//! Formal, fail-closed promotion gate for Switchboard extension categories.
//!
//! Categories share evidence metadata, not an execution trait. Each category
//! keeps a distinct contract so endpoint, telemetry, context, client-writing,
//! and optimization capabilities cannot be treated as interchangeable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProvenanceEvidence {
    pub source_url: String,
    pub source_revision: String,
    pub artifact_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LicenseEvidence {
    pub spdx_id: String,
    pub license_source_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub(crate) enum NetworkDeclaration {
    Missing,
    NoNetwork {
        deterministic_verification_fixture: Option<String>,
    },
    DeclaredDestinations {
        destinations: Vec<String>,
        explicit_user_opt_in: bool,
        secrets_redacted: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeterministicFixtureEvidence {
    pub fixture_count: u32,
    pub deterministic_replay: bool,
    pub fixture_bundle_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QualityEvidence {
    pub measured: bool,
    pub sample_count: u64,
    pub successful_task_rate_basis_points: u16,
    pub wrong_omission_rate_basis_points: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub(crate) enum StateEffect {
    ReadOnly,
    WritesState {
        rollback_contract: Option<String>,
        uninstall_contract: Option<String>,
        deterministic_write_verification_fixture: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginManifest {
    pub id: String,
    pub version_pin: String,
    pub provenance: Option<ProvenanceEvidence>,
    pub license: Option<LicenseEvidence>,
    pub network: NetworkDeclaration,
    pub deterministic_fixtures: Option<DeterministicFixtureEvidence>,
    pub quality: Option<QualityEvidence>,
    pub state_effect: StateEffect,
    pub update_source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationEnginePlugin {
    pub manifest: PluginManifest,
    pub lifecycle_control: bool,
    pub request_transform_contract: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationAddonPlugin {
    pub manifest: PluginManifest,
    pub bounded_action: String,
    pub host_engine_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextProviderPlugin {
    pub manifest: PluginManifest,
    pub read_only_context: bool,
    pub context_pack_schema: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodingClientAdapterPlugin {
    pub manifest: PluginManifest,
    pub client_id: String,
    pub detect_plan_apply_verify_contract: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InferenceEndpointProfilePlugin {
    pub manifest: PluginManifest,
    pub protocol: String,
    pub explicit_user_enrollment: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetryExporterPlugin {
    pub manifest: PluginManifest,
    pub content_free_schema: bool,
    pub local_only_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "category", content = "contract")]
pub(crate) enum PluginCandidate {
    OptimizationEngine(OptimizationEnginePlugin),
    OptimizationAddon(OptimizationAddonPlugin),
    ContextProvider(ContextProviderPlugin),
    CodingClientAdapter(CodingClientAdapterPlugin),
    InferenceEndpointProfile(InferenceEndpointProfilePlugin),
    TelemetryExporter(TelemetryExporterPlugin),
}

impl PluginCandidate {
    fn manifest(&self) -> &PluginManifest {
        match self {
            Self::OptimizationEngine(value) => &value.manifest,
            Self::OptimizationAddon(value) => &value.manifest,
            Self::ContextProvider(value) => &value.manifest,
            Self::CodingClientAdapter(value) => &value.manifest,
            Self::InferenceEndpointProfile(value) => &value.manifest,
            Self::TelemetryExporter(value) => &value.manifest,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PluginPromotionPolicy {
    pub allowed_spdx_licenses: BTreeSet<String>,
    pub minimum_quality_samples: u64,
    pub minimum_successful_task_rate_basis_points: u16,
    pub maximum_wrong_omission_rate_basis_points: u16,
}

impl Default for PluginPromotionPolicy {
    fn default() -> Self {
        Self {
            allowed_spdx_licenses: BTreeSet::from([
                "Apache-2.0".to_string(),
                "BSD-2-Clause".to_string(),
                "BSD-3-Clause".to_string(),
                "MIT".to_string(),
                "MPL-2.0".to_string(),
            ]),
            minimum_quality_samples: 30,
            minimum_successful_task_rate_basis_points: 9_800,
            maximum_wrong_omission_rate_basis_points: 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PromotionRejection {
    MissingPluginId,
    MissingProvenance,
    InvalidProvenanceSource,
    MissingSourceRevision,
    InvalidSourceRevision,
    InvalidArtifactDigest,
    MissingLicense,
    InvalidLicenseSource,
    LicenseNotAllowed,
    MissingNetworkDeclaration,
    MissingNoNetworkVerification,
    MissingNetworkDestination,
    InsecureNetworkDestination,
    NetworkOptInRequired,
    NetworkSecretRedactionRequired,
    MissingDeterministicFixtures,
    FixtureReplayNotDeterministic,
    InvalidFixtureDigest,
    MissingQualityEvidence,
    QualityNotMeasured,
    InsufficientQualitySamples,
    SuccessfulTaskRateBelowThreshold,
    WrongOmissionRateAboveThreshold,
    InvalidQualityRate,
    MissingRollbackContract,
    MissingUninstallContract,
    MissingWriteVerificationFixture,
    InvalidVersionPin,
    MissingUpdateSource,
    InsecureUpdateSource,
    MissingCategoryContract,
    ContextProviderMustBeReadOnly,
    EndpointRequiresExplicitEnrollment,
    TelemetryMustBeContentFree,
    TelemetryMustSupportLocalOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginPromotionDecision {
    pub promoted: bool,
    pub rejections: Vec<PromotionRejection>,
}

pub(crate) fn evaluate_plugin_promotion(
    candidate: &PluginCandidate,
    policy: &PluginPromotionPolicy,
) -> PluginPromotionDecision {
    let manifest = candidate.manifest();
    let mut rejections = Vec::new();
    if manifest.id.trim().is_empty() {
        rejections.push(PromotionRejection::MissingPluginId);
    }
    validate_provenance(manifest, &mut rejections);
    validate_license(manifest, policy, &mut rejections);
    validate_network(manifest, &mut rejections);
    validate_fixtures(manifest, &mut rejections);
    validate_quality(manifest, policy, &mut rejections);
    validate_state_effect(manifest, &mut rejections);
    if !valid_version_pin(&manifest.version_pin) {
        rejections.push(PromotionRejection::InvalidVersionPin);
    }
    match manifest.update_source.as_deref() {
        None => rejections.push(PromotionRejection::MissingUpdateSource),
        Some(value) if !valid_https_url(value) => {
            rejections.push(PromotionRejection::InsecureUpdateSource)
        }
        Some(_) => {}
    }
    validate_category(candidate, &mut rejections);
    rejections.sort_by_key(|value| format!("{value:?}"));
    rejections.dedup();
    PluginPromotionDecision {
        promoted: rejections.is_empty(),
        rejections,
    }
}

fn validate_provenance(manifest: &PluginManifest, rejections: &mut Vec<PromotionRejection>) {
    let Some(evidence) = &manifest.provenance else {
        rejections.push(PromotionRejection::MissingProvenance);
        return;
    };
    if !valid_https_url(&evidence.source_url) {
        rejections.push(PromotionRejection::InvalidProvenanceSource);
    }
    if evidence.source_revision.trim().is_empty() {
        rejections.push(PromotionRejection::MissingSourceRevision);
    } else if !valid_source_revision(&evidence.source_revision) {
        rejections.push(PromotionRejection::InvalidSourceRevision);
    }
    if !valid_sha256(&evidence.artifact_sha256) {
        rejections.push(PromotionRejection::InvalidArtifactDigest);
    }
}

fn validate_license(
    manifest: &PluginManifest,
    policy: &PluginPromotionPolicy,
    rejections: &mut Vec<PromotionRejection>,
) {
    let Some(evidence) = &manifest.license else {
        rejections.push(PromotionRejection::MissingLicense);
        return;
    };
    if !valid_https_url(&evidence.license_source_url) {
        rejections.push(PromotionRejection::InvalidLicenseSource);
    }
    if !policy.allowed_spdx_licenses.contains(&evidence.spdx_id) {
        rejections.push(PromotionRejection::LicenseNotAllowed);
    }
}

fn validate_network(manifest: &PluginManifest, rejections: &mut Vec<PromotionRejection>) {
    match &manifest.network {
        NetworkDeclaration::Missing => {
            rejections.push(PromotionRejection::MissingNetworkDeclaration)
        }
        NetworkDeclaration::NoNetwork {
            deterministic_verification_fixture,
        } => {
            if deterministic_verification_fixture
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                rejections.push(PromotionRejection::MissingNoNetworkVerification);
            }
        }
        NetworkDeclaration::DeclaredDestinations {
            destinations,
            explicit_user_opt_in,
            secrets_redacted,
        } => {
            if destinations.is_empty() {
                rejections.push(PromotionRejection::MissingNetworkDestination);
            }
            if destinations.iter().any(|value| !valid_https_url(value)) {
                rejections.push(PromotionRejection::InsecureNetworkDestination);
            }
            if !explicit_user_opt_in {
                rejections.push(PromotionRejection::NetworkOptInRequired);
            }
            if !secrets_redacted {
                rejections.push(PromotionRejection::NetworkSecretRedactionRequired);
            }
        }
    }
}

fn validate_fixtures(manifest: &PluginManifest, rejections: &mut Vec<PromotionRejection>) {
    let Some(evidence) = &manifest.deterministic_fixtures else {
        rejections.push(PromotionRejection::MissingDeterministicFixtures);
        return;
    };
    if evidence.fixture_count == 0 || !evidence.deterministic_replay {
        rejections.push(PromotionRejection::FixtureReplayNotDeterministic);
    }
    if !valid_sha256(&evidence.fixture_bundle_sha256) {
        rejections.push(PromotionRejection::InvalidFixtureDigest);
    }
}

fn validate_quality(
    manifest: &PluginManifest,
    policy: &PluginPromotionPolicy,
    rejections: &mut Vec<PromotionRejection>,
) {
    let Some(evidence) = &manifest.quality else {
        rejections.push(PromotionRejection::MissingQualityEvidence);
        return;
    };
    if !evidence.measured {
        rejections.push(PromotionRejection::QualityNotMeasured);
    }
    if evidence.successful_task_rate_basis_points > 10_000
        || evidence.wrong_omission_rate_basis_points > 10_000
    {
        rejections.push(PromotionRejection::InvalidQualityRate);
    }
    if evidence.sample_count < policy.minimum_quality_samples {
        rejections.push(PromotionRejection::InsufficientQualitySamples);
    }
    if evidence.successful_task_rate_basis_points < policy.minimum_successful_task_rate_basis_points
    {
        rejections.push(PromotionRejection::SuccessfulTaskRateBelowThreshold);
    }
    if evidence.wrong_omission_rate_basis_points > policy.maximum_wrong_omission_rate_basis_points {
        rejections.push(PromotionRejection::WrongOmissionRateAboveThreshold);
    }
}

fn validate_state_effect(manifest: &PluginManifest, rejections: &mut Vec<PromotionRejection>) {
    if let StateEffect::WritesState {
        rollback_contract,
        uninstall_contract,
        deterministic_write_verification_fixture,
    } = &manifest.state_effect
    {
        if missing(rollback_contract) {
            rejections.push(PromotionRejection::MissingRollbackContract);
        }
        if missing(uninstall_contract) {
            rejections.push(PromotionRejection::MissingUninstallContract);
        }
        if missing(deterministic_write_verification_fixture) {
            rejections.push(PromotionRejection::MissingWriteVerificationFixture);
        }
    }
}

fn validate_category(candidate: &PluginCandidate, rejections: &mut Vec<PromotionRejection>) {
    match candidate {
        PluginCandidate::OptimizationEngine(value) => {
            if !value.lifecycle_control || value.request_transform_contract.trim().is_empty() {
                rejections.push(PromotionRejection::MissingCategoryContract);
            }
        }
        PluginCandidate::OptimizationAddon(value) => {
            if value.bounded_action.trim().is_empty() || !value.host_engine_required {
                rejections.push(PromotionRejection::MissingCategoryContract);
            }
        }
        PluginCandidate::ContextProvider(value) => {
            if !value.read_only_context {
                rejections.push(PromotionRejection::ContextProviderMustBeReadOnly);
            }
            if value.context_pack_schema.trim().is_empty() {
                rejections.push(PromotionRejection::MissingCategoryContract);
            }
        }
        PluginCandidate::CodingClientAdapter(value) => {
            if value.client_id.trim().is_empty() || !value.detect_plan_apply_verify_contract {
                rejections.push(PromotionRejection::MissingCategoryContract);
            }
        }
        PluginCandidate::InferenceEndpointProfile(value) => {
            if value.protocol.trim().is_empty() {
                rejections.push(PromotionRejection::MissingCategoryContract);
            }
            if !value.explicit_user_enrollment {
                rejections.push(PromotionRejection::EndpointRequiresExplicitEnrollment);
            }
        }
        PluginCandidate::TelemetryExporter(value) => {
            if !value.content_free_schema {
                rejections.push(PromotionRejection::TelemetryMustBeContentFree);
            }
            if !value.local_only_supported {
                rejections.push(PromotionRejection::TelemetryMustSupportLocalOnly);
            }
        }
    }
}

fn missing(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|value| value.trim().is_empty())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_https_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

fn valid_source_revision(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    value == value.trim()
        && value.len() >= 7
        && !matches!(
            normalized.as_str(),
            "latest" | "main" | "master" | "dev" | "head"
        )
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'/' | b'+')
        })
}

fn valid_version_pin(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    !value.trim().is_empty()
        && value == value.trim()
        && !["latest", "main", "master", "dev"]
            .iter()
            .any(|floating| lowercase == *floating)
        && !value.contains(['*', '^', '~', '>', '<', '='])
        && value.bytes().any(|byte| byte.is_ascii_digit())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PluginCandidate {
        serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/plugin-promotion-candidate.json"
        ))
        .unwrap()
    }

    #[test]
    fn deterministic_fixture_passes_all_formal_gates() {
        let decision = evaluate_plugin_promotion(&fixture(), &PluginPromotionPolicy::default());
        assert_eq!(
            decision,
            PluginPromotionDecision {
                promoted: true,
                rejections: vec![]
            }
        );
    }

    #[test]
    fn writing_plugin_requires_rollback_uninstall_and_write_verification() {
        let mut candidate = fixture();
        let PluginCandidate::CodingClientAdapter(value) = &mut candidate else {
            panic!("fixture category")
        };
        value.manifest.state_effect = StateEffect::WritesState {
            rollback_contract: None,
            uninstall_contract: None,
            deterministic_write_verification_fixture: None,
        };
        let decision = evaluate_plugin_promotion(&candidate, &PluginPromotionPolicy::default());
        assert!(decision
            .rejections
            .contains(&PromotionRejection::MissingRollbackContract));
        assert!(decision
            .rejections
            .contains(&PromotionRejection::MissingUninstallContract));
        assert!(decision
            .rejections
            .contains(&PromotionRejection::MissingWriteVerificationFixture));
    }

    #[test]
    fn missing_provenance_license_network_fixture_quality_pin_and_update_fail_closed() {
        let mut candidate = fixture();
        let PluginCandidate::CodingClientAdapter(value) = &mut candidate else {
            panic!("fixture category")
        };
        value.manifest.provenance = None;
        value.manifest.license = None;
        value.manifest.network = NetworkDeclaration::Missing;
        value.manifest.deterministic_fixtures = None;
        value.manifest.quality = None;
        value.manifest.version_pin = "latest".into();
        value.manifest.update_source = None;
        let decision = evaluate_plugin_promotion(&candidate, &PluginPromotionPolicy::default());
        for expected in [
            PromotionRejection::MissingProvenance,
            PromotionRejection::MissingLicense,
            PromotionRejection::MissingNetworkDeclaration,
            PromotionRejection::MissingDeterministicFixtures,
            PromotionRejection::MissingQualityEvidence,
            PromotionRejection::InvalidVersionPin,
            PromotionRejection::MissingUpdateSource,
        ] {
            assert!(
                decision.rejections.contains(&expected),
                "missing {expected:?}"
            );
        }
    }

    #[test]
    fn quality_and_wrong_omission_thresholds_are_independent_gates() {
        let mut candidate = fixture();
        let PluginCandidate::CodingClientAdapter(value) = &mut candidate else {
            panic!("fixture category")
        };
        let quality = value.manifest.quality.as_mut().unwrap();
        quality.successful_task_rate_basis_points = 9_700;
        quality.wrong_omission_rate_basis_points = 200;
        let decision = evaluate_plugin_promotion(&candidate, &PluginPromotionPolicy::default());
        assert!(decision
            .rejections
            .contains(&PromotionRejection::SuccessfulTaskRateBelowThreshold));
        assert!(decision
            .rejections
            .contains(&PromotionRejection::WrongOmissionRateAboveThreshold));
    }

    #[test]
    fn category_contracts_remain_distinct_and_apply_category_specific_safety() {
        let manifest = match fixture() {
            PluginCandidate::CodingClientAdapter(value) => value.manifest,
            _ => unreachable!(),
        };
        let context = PluginCandidate::ContextProvider(ContextProviderPlugin {
            manifest: manifest.clone(),
            read_only_context: false,
            context_pack_schema: "v1".into(),
        });
        let telemetry = PluginCandidate::TelemetryExporter(TelemetryExporterPlugin {
            manifest,
            content_free_schema: false,
            local_only_supported: false,
        });
        let context_decision =
            evaluate_plugin_promotion(&context, &PluginPromotionPolicy::default());
        assert!(context_decision
            .rejections
            .contains(&PromotionRejection::ContextProviderMustBeReadOnly));
        let telemetry_decision =
            evaluate_plugin_promotion(&telemetry, &PluginPromotionPolicy::default());
        assert!(telemetry_decision
            .rejections
            .contains(&PromotionRejection::TelemetryMustBeContentFree));
        assert!(telemetry_decision
            .rejections
            .contains(&PromotionRejection::TelemetryMustSupportLocalOnly));
    }

    #[test]
    fn all_six_explicit_categories_use_distinct_contracts() {
        let manifest = match fixture() {
            PluginCandidate::CodingClientAdapter(value) => value.manifest,
            _ => unreachable!(),
        };
        let candidates = [
            PluginCandidate::OptimizationEngine(OptimizationEnginePlugin {
                manifest: manifest.clone(),
                lifecycle_control: true,
                request_transform_contract: "transform-v1".into(),
            }),
            PluginCandidate::OptimizationAddon(OptimizationAddonPlugin {
                manifest: manifest.clone(),
                bounded_action: "cache-order".into(),
                host_engine_required: true,
            }),
            PluginCandidate::ContextProvider(ContextProviderPlugin {
                manifest: manifest.clone(),
                read_only_context: true,
                context_pack_schema: "context-pack-v1".into(),
            }),
            PluginCandidate::CodingClientAdapter(CodingClientAdapterPlugin {
                manifest: manifest.clone(),
                client_id: "fixture".into(),
                detect_plan_apply_verify_contract: true,
            }),
            PluginCandidate::InferenceEndpointProfile(InferenceEndpointProfilePlugin {
                manifest: manifest.clone(),
                protocol: "openai-compatible".into(),
                explicit_user_enrollment: true,
            }),
            PluginCandidate::TelemetryExporter(TelemetryExporterPlugin {
                manifest,
                content_free_schema: true,
                local_only_supported: true,
            }),
        ];
        assert_eq!(candidates.len(), 6);
        assert!(candidates.iter().all(|candidate| evaluate_plugin_promotion(
            candidate,
            &PluginPromotionPolicy::default()
        )
        .promoted));
    }
}
