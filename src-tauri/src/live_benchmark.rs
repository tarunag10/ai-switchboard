//! Offline-safe contract for live endpoint benchmark adapters.
//!
//! The mock matrix proves the harness shape without starting a runtime or
//! making a network request. Runtime-specific adapters remain future work.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::inference_endpoint::InferenceEndpoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum BenchmarkVariant {
    /// Switchboard off, runtime baseline.
    B00,
    /// Switchboard on, runtime baseline.
    B10,
    /// Switchboard off, runtime optimization on.
    B01,
    /// Switchboard on, runtime optimization on.
    B11,
}

impl BenchmarkVariant {
    pub const ALL: [Self; 4] = [Self::B00, Self::B10, Self::B01, Self::B11];

    pub fn switchboard_optimization(self) -> bool {
        matches!(self, Self::B10 | Self::B11)
    }

    pub fn runtime_optimization(self) -> bool {
        matches!(self, Self::B01 | Self::B11)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BenchmarkCase {
    pub id: String,
    pub input: String,
    pub variant: BenchmarkVariant,
}

impl BenchmarkCase {
    fn with_variant(&self, variant: BenchmarkVariant) -> Self {
        Self {
            id: self.id.clone(),
            input: self.input.clone(),
            variant,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BenchmarkResult {
    pub target_id: String,
    pub endpoint_id: String,
    pub case_id: String,
    pub variant: BenchmarkVariant,
    pub successful: bool,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub end_to_end_latency_ms: f64,
    pub optimization_latency_ms: f64,
    pub quality_label: String,
}

pub(crate) trait LiveBenchmarkTarget {
    fn id(&self) -> &str;
    fn endpoint(&self) -> &dyn InferenceEndpoint;
    fn warmup(&self) -> Result<()>;
    fn run_case(&self, case: BenchmarkCase) -> Result<BenchmarkResult>;
}

/// Runs the required variants in stable B00, B10, B01, B11 order.
pub(crate) fn run_2x2_matrix(
    target: &dyn LiveBenchmarkTarget,
    case: &BenchmarkCase,
) -> Result<Vec<BenchmarkResult>> {
    target.warmup()?;
    BenchmarkVariant::ALL
        .into_iter()
        .map(|variant| target.run_case(case.with_variant(variant)))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use anyhow::{ensure, Result};

    use super::*;
    use crate::inference_endpoint::{InferenceCapabilities, InferenceEndpoint, InferenceProtocol};

    struct MockEndpoint;

    impl InferenceEndpoint for MockEndpoint {
        fn id(&self) -> &str {
            "mock-local"
        }

        fn display_name(&self) -> &str {
            "Deterministic local mock"
        }

        fn protocol(&self) -> InferenceProtocol {
            InferenceProtocol::LocalMock
        }

        fn capabilities(&self) -> InferenceCapabilities {
            InferenceCapabilities {
                streaming: false,
                tool_use: false,
                runtime_optimization: true,
            }
        }
    }

    struct MockTarget {
        endpoint: MockEndpoint,
        warmed: AtomicBool,
    }

    impl MockTarget {
        fn new() -> Self {
            Self {
                endpoint: MockEndpoint,
                warmed: AtomicBool::new(false),
            }
        }
    }

    impl LiveBenchmarkTarget for MockTarget {
        fn id(&self) -> &str {
            "mock-target"
        }

        fn endpoint(&self) -> &dyn InferenceEndpoint {
            &self.endpoint
        }

        fn warmup(&self) -> Result<()> {
            self.warmed.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn run_case(&self, case: BenchmarkCase) -> Result<BenchmarkResult> {
            ensure!(self.warmed.load(Ordering::SeqCst), "target was not warmed");
            let switchboard = case.variant.switchboard_optimization();
            let runtime = case.variant.runtime_optimization();
            Ok(BenchmarkResult {
                target_id: self.id().to_string(),
                endpoint_id: self.endpoint().id().to_string(),
                case_id: case.id,
                variant: case.variant,
                successful: true,
                input_tokens: if switchboard { 75 } else { 100 },
                output_tokens: 20,
                end_to_end_latency_ms: if runtime { 80.0 } else { 100.0 },
                optimization_latency_ms: if switchboard { 2.0 } else { 0.0 },
                quality_label: "mock_exact".to_string(),
            })
        }
    }

    fn sample_case() -> BenchmarkCase {
        BenchmarkCase {
            id: "format-small-file".to_string(),
            input: "Format this deterministic fixture".to_string(),
            variant: BenchmarkVariant::B00,
        }
    }

    #[test]
    fn local_mock_runs_the_complete_2x2_matrix_deterministically() {
        let target = MockTarget::new();
        let first = run_2x2_matrix(&target, &sample_case()).unwrap();
        let second = run_2x2_matrix(&target, &sample_case()).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|result| result.variant)
                .collect::<Vec<_>>(),
            BenchmarkVariant::ALL
        );
        assert_eq!(first.iter().filter(|result| result.successful).count(), 4);
        assert_eq!(first[0].input_tokens, 100);
        assert_eq!(first[1].input_tokens, 75);
        assert_eq!(first[2].end_to_end_latency_ms, 80.0);
        assert_eq!(first[3].input_tokens, 75);
        assert_eq!(first[3].end_to_end_latency_ms, 80.0);
        assert_eq!(target.endpoint().protocol(), InferenceProtocol::LocalMock);
        assert!(target.endpoint().capabilities().runtime_optimization);
    }

    #[test]
    fn target_refuses_a_case_before_warmup() {
        let target = MockTarget::new();
        let error = target.run_case(sample_case()).unwrap_err();
        assert_eq!(error.to_string(), "target was not warmed");
    }
}
