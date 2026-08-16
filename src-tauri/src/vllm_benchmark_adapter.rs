//! Developer-only normalization of externally collected vLLM benchmark evidence.
//!
//! This module deliberately does not launch AIPerf, query an endpoint, import a
//! Python package, or inspect a GPU. It converts supplied AIPerf/runtime-native
//! JSON into one stable evidence record that can be stored with a benchmark run.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VllmBenchmarkSource {
    Aiperf,
    RuntimeNative,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VllmBenchmarkEvidence {
    pub schema_version: u8,
    pub source: VllmBenchmarkSource,
    /// Collector version reported by the imported artifact. Absence is kept as
    /// `None`; it is never guessed from the adapter's compatibility target.
    pub source_version: Option<String>,
    pub runtime: String,
    pub model: String,
    pub ttft_ms: f64,
    pub itl_ms: Option<f64>,
    pub tpot_ms: Option<f64>,
    pub e2e_latency_ms: f64,
    pub throughput_tokens_per_second: f64,
    pub prefix_cache_hit_rate: Option<f64>,
    pub queue_depth: Option<f64>,
    pub queue_time_ms: Option<f64>,
    pub gpu_utilization_pct: Option<f64>,
    pub gpu_memory_used_bytes: Option<u64>,
    pub developer_mode: bool,
}

impl VllmBenchmarkEvidence {
    fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("ttft_ms", Some(self.ttft_ms)),
            ("itl_ms", self.itl_ms),
            ("tpot_ms", self.tpot_ms),
            ("e2e_latency_ms", Some(self.e2e_latency_ms)),
            (
                "throughput_tokens_per_second",
                Some(self.throughput_tokens_per_second),
            ),
            ("queue_depth", self.queue_depth),
            ("queue_time_ms", self.queue_time_ms),
        ] {
            if value.is_some_and(|candidate| !candidate.is_finite() || candidate < 0.0) {
                bail!("invalid {name}: expected a finite non-negative number");
            }
        }
        if self
            .prefix_cache_hit_rate
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            bail!("invalid prefix_cache_hit_rate: expected a value from 0 to 1");
        }
        if self
            .gpu_utilization_pct
            .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
        {
            bail!("invalid gpu_utilization_pct: expected a value from 0 to 100");
        }
        Ok(())
    }
}

/// Normalize externally collected JSON. The caller must opt into developer
/// mode so benchmark evidence cannot accidentally become a production probe.
pub(crate) fn ingest_vllm_benchmark(
    source: VllmBenchmarkSource,
    input: &Value,
    developer_mode: bool,
) -> Result<VllmBenchmarkEvidence> {
    if !developer_mode {
        bail!("vLLM benchmark ingestion is available only in developer mode");
    }

    let runtime =
        text(input, &["runtime", "metadata.runtime"]).unwrap_or_else(|| "vllm".to_string());
    let model = text(input, &["model", "model_name", "metadata.model"])
        .ok_or_else(|| anyhow!("benchmark evidence is missing model identity"))?;

    let evidence = VllmBenchmarkEvidence {
        schema_version: 1,
        source,
        source_version: text(
            input,
            &[
                "aiperf_version",
                "metadata.aiperf_version",
                "runtime_version",
                "metadata.runtime_version",
            ],
        ),
        runtime,
        model,
        ttft_ms: required_number(
            input,
            &[
                "metrics.time_to_first_token.value",
                "ttft_ms",
                "metrics.ttft_ms",
                "time_to_first_token_ms",
            ],
            "TTFT",
        )?,
        itl_ms: number(
            input,
            &[
                "metrics.inter_token_latency.value",
                "itl_ms",
                "metrics.itl_ms",
                "inter_token_latency_ms",
            ],
        ),
        tpot_ms: number(
            input,
            &[
                "metrics.time_per_output_token.value",
                "tpot_ms",
                "metrics.tpot_ms",
                "time_per_output_token_ms",
            ],
        ),
        e2e_latency_ms: required_number(
            input,
            &[
                "e2e_latency_ms",
                "metrics.request_latency.value",
                "metrics.e2e_latency_ms",
                "request_latency_ms",
            ],
            "end-to-end latency",
        )?,
        throughput_tokens_per_second: required_number(
            input,
            &[
                "throughput_tokens_per_second",
                "metrics.output_token_throughput_per_user.value",
                "metrics.output_token_throughput.value",
                "metrics.throughput_tokens_per_second",
                "output_token_throughput",
            ],
            "throughput",
        )?,
        prefix_cache_hit_rate: number(
            input,
            &[
                "prefix_cache_hit_rate",
                "metrics.prefix_cache_hit_rate",
                "cache.prefix_hit_rate",
            ],
        ),
        queue_depth: number(
            input,
            &["queue_depth", "metrics.queue_depth", "queue.depth"],
        ),
        queue_time_ms: number(
            input,
            &["queue_time_ms", "metrics.queue_time_ms", "queue.time_ms"],
        ),
        gpu_utilization_pct: number(
            input,
            &[
                "gpu_utilization_pct",
                "metrics.gpu_utilization_pct",
                "gpu.utilization_pct",
            ],
        ),
        gpu_memory_used_bytes: unsigned_integer(
            input,
            &[
                "gpu_memory_used_bytes",
                "metrics.gpu_memory_used_bytes",
                "gpu.memory_used_bytes",
            ],
        ),
        developer_mode,
    };
    evidence.validate()?;
    Ok(evidence)
}

fn at_path<'a>(input: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(input, |value, segment| value.get(segment))
}

fn text(input: &Value, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| at_path(input, path).and_then(Value::as_str))
        .map(str::to_string)
}

fn number(input: &Value, paths: &[&str]) -> Option<f64> {
    paths
        .iter()
        .find_map(|path| at_path(input, path).and_then(Value::as_f64))
}

fn unsigned_integer(input: &Value, paths: &[&str]) -> Option<u64> {
    paths
        .iter()
        .find_map(|path| at_path(input, path).and_then(Value::as_u64))
}

fn required_number(input: &Value, paths: &[&str], label: &str) -> Result<f64> {
    number(input, paths).ok_or_else(|| anyhow!("benchmark evidence is missing {label}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aiperf_fixture_into_stable_schema() {
        let input: Value = serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/vllm-aiperf-evidence.json"
        ))
        .expect("fixture");
        let evidence = ingest_vllm_benchmark(VllmBenchmarkSource::Aiperf, &input, true)
            .expect("normalized evidence");

        assert_eq!(evidence.model, "fixture/model");
        assert_eq!(evidence.source_version.as_deref(), Some("0.12.0"));
        assert_eq!(evidence.ttft_ms, 112.5);
        assert_eq!(evidence.itl_ms, Some(8.25));
        assert_eq!(evidence.tpot_ms, Some(9.5));
        assert_eq!(evidence.prefix_cache_hit_rate, Some(0.72));
        assert_eq!(evidence.queue_depth, Some(2.0));
        assert_eq!(evidence.gpu_utilization_pct, Some(61.0));
    }

    #[test]
    fn runtime_native_evidence_allows_optional_gpu_metrics_to_be_absent() {
        let input: Value = serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/vllm-runtime-evidence.json"
        ))
        .expect("fixture");
        let evidence = ingest_vllm_benchmark(VllmBenchmarkSource::RuntimeNative, &input, true)
            .expect("normalized evidence");

        assert_eq!(evidence.runtime, "vllm-openai");
        assert_eq!(evidence.gpu_utilization_pct, None);
        assert_eq!(evidence.gpu_memory_used_bytes, None);
        assert_eq!(evidence.queue_time_ms, Some(4.0));
    }

    #[test]
    fn rejects_non_developer_ingestion_and_invalid_rates() {
        let input = serde_json::json!({
            "model": "fixture/model",
            "ttft_ms": 1.0,
            "e2e_latency_ms": 2.0,
            "throughput_tokens_per_second": 3.0,
            "prefix_cache_hit_rate": 1.2
        });
        assert!(
            ingest_vllm_benchmark(VllmBenchmarkSource::Aiperf, &input, false)
                .expect_err("developer mode gate")
                .to_string()
                .contains("developer mode")
        );
        assert!(
            ingest_vllm_benchmark(VllmBenchmarkSource::Aiperf, &input, true)
                .expect_err("invalid rate")
                .to_string()
                .contains("prefix_cache_hit_rate")
        );
    }

    #[test]
    fn requires_core_latency_and_throughput_evidence() {
        let input = serde_json::json!({ "model": "fixture/model" });
        assert!(
            ingest_vllm_benchmark(VllmBenchmarkSource::RuntimeNative, &input, true)
                .expect_err("required TTFT")
                .to_string()
                .contains("TTFT")
        );
    }
}
