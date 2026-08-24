use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::digest::{bool_flags, bounded_digest, is_lowercase_sha256};
use crate::ProtocolError;

pub const PROTOCOL_VERSION: u16 = 1;
pub const FRAME_PREFIX_BYTES: usize = 4;
pub const MAX_FRAME_BYTES: usize = 4_096;
pub const MAX_PAYLOAD_BYTES: usize = MAX_FRAME_BYTES - FRAME_PREFIX_BYTES;
pub const MAX_IDENTIFIER_BYTES: usize = 128;
const HOST_CONTRACT_SCHEMA_VERSION: u32 = 1;
const COLLECTION_RECEIPT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreparationMessageKind {
    PrepareNoProcess,
}

impl PreparationMessageKind {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::PrepareNoProcess => "prepare_no_process",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HelperAction {
    CodexVersionProbeV1,
}

impl HelperAction {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::CodexVersionProbeV1 => "codex_version_probe_v1",
        }
    }

    const fn host_value(self) -> &'static str {
        match self {
            Self::CodexVersionProbeV1 => "codex-version-probe-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LaunchAuthority {
    PreparationOnly,
}

impl LaunchAuthority {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::PreparationOnly => "preparation_only",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HelperBoundary {
    SeparatelySignedNestedHelperRequired,
}

impl HelperBoundary {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::SeparatelySignedNestedHelperRequired => {
                "separately_signed_nested_helper_required"
            }
        }
    }

    const fn host_value(self) -> &'static str {
        match self {
            Self::SeparatelySignedNestedHelperRequired => {
                "separately-signed-nested-helper-required"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollectionProvenance {
    CollectedNpmSchemaV2,
}

impl CollectionProvenance {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::CollectedNpmSchemaV2 => "collected_npm_schema_v2",
        }
    }

    const fn host_value(self) -> &'static str {
        match self {
            Self::CollectedNpmSchemaV2 => "collected-npm-schema-v2",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContainmentProfile {
    MacosRestrictedHelperV1,
}

impl ContainmentProfile {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::MacosRestrictedHelperV1 => "macos_restricted_helper_v1",
        }
    }

    const fn host_value(self) -> &'static str {
        match self {
            Self::MacosRestrictedHelperV1 => "macos-restricted-helper-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTraffic {
    None,
}

impl ProviderTraffic {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreparationResultState {
    ShapeConsistentNoProcess,
}

impl PreparationResultState {
    const fn digest_value(self) -> &'static str {
        match self {
            Self::ShapeConsistentNoProcess => "shape_consistent_no_process",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationRequestFrame {
    pub protocol_version: u16,
    pub message_kind: PreparationMessageKind,
    pub action: HelperAction,
    pub authority: LaunchAuthority,
    pub helper_boundary: HelperBoundary,
    pub target_provenance: CollectionProvenance,
    pub containment_profile: ContainmentProfile,
    pub host_contract_schema_version: u32,
    pub collection_receipt_schema_version: u32,
    pub request_id: String,
    pub request_digest: String,
    pub binding_digest: String,
    pub preparation_receipt_id: String,
    pub preparation_receipt_digest: String,
    pub prepared_at: String,
    pub session_id: String,
    pub session_snapshot_digest: String,
    pub workspace_digest: String,
    pub plan_id: String,
    pub plan_snapshot_digest: String,
    pub process_run_id: String,
    pub process_run_spec_digest: String,
    pub grant_id: String,
    pub grant_receipt_digest: String,
    pub admission_id: String,
    pub admission_receipt_digest: String,
    pub attempt_id: String,
    pub probe_plan_digest: String,
    pub candidate_id: String,
    pub launcher_chain_identity_digest: String,
    pub preflight_identity_digest: String,
    pub containment_identity_digest: String,
    pub host_instance_identity_digest: String,
    pub boot_session_identity_digest: String,
    pub os_build_identity_digest: String,
    pub helper_code_identity_digest: String,
    pub helper_entitlements_identity_digest: String,
    pub enforcement_policy_identity_digest: String,
    pub manual_opt_in_required: bool,
    pub runnable: bool,
    pub supported: bool,
    pub process_start_enabled: bool,
    pub execution_reserved: bool,
    pub provider_traffic: ProviderTraffic,
    pub user_workspace_writes_enabled: bool,
    pub frame_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationResponseFrame {
    pub protocol_version: u16,
    pub message_kind: PreparationMessageKind,
    pub action: HelperAction,
    pub request_frame_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub binding_digest: String,
    pub preparation_receipt_id: String,
    pub preparation_receipt_digest: String,
    pub attempt_id: String,
    pub state: PreparationResultState,
    pub host_authenticated: bool,
    pub freshness_verified: bool,
    pub launch_authority_verified: bool,
    pub helper_invoked: bool,
    pub process_started: bool,
    pub execution_reserved: bool,
    pub runnable: bool,
    pub supported: bool,
    pub provider_traffic: ProviderTraffic,
    pub user_workspace_writes_enabled: bool,
    pub frame_digest: String,
}

/// Construction-only projection of the host's complete opaque preparation transcript.
///
/// Internal consistency is independently recomputed, but the transcript remains
/// unauthenticated and does not establish freshness or launch authority.
pub struct HostPreparationProjection<'a> {
    pub request_id: &'a str,
    pub request_digest: &'a str,
    pub binding_digest: &'a str,
    pub preparation_receipt_id: &'a str,
    pub preparation_receipt_digest: &'a str,
    pub prepared_at: &'a str,
    pub session_id: &'a str,
    pub session_snapshot_digest: &'a str,
    pub workspace_digest: &'a str,
    pub plan_id: &'a str,
    pub plan_snapshot_digest: &'a str,
    pub process_run_id: &'a str,
    pub process_run_spec_digest: &'a str,
    pub grant_id: &'a str,
    pub grant_receipt_digest: &'a str,
    pub admission_id: &'a str,
    pub admission_receipt_digest: &'a str,
    pub attempt_id: &'a str,
    pub probe_plan_digest: &'a str,
    pub candidate_id: &'a str,
    pub launcher_chain_identity_digest: &'a str,
    pub preflight_identity_digest: &'a str,
    pub containment_identity_digest: &'a str,
    pub host_instance_identity_digest: &'a str,
    pub boot_session_identity_digest: &'a str,
    pub os_build_identity_digest: &'a str,
    pub helper_code_identity_digest: &'a str,
    pub helper_entitlements_identity_digest: &'a str,
    pub enforcement_policy_identity_digest: &'a str,
}

pub fn preparation_request_from_host(
    value: HostPreparationProjection<'_>,
) -> Result<PreparationRequestFrame, ProtocolError> {
    let mut request = PreparationRequestFrame {
        protocol_version: PROTOCOL_VERSION,
        message_kind: PreparationMessageKind::PrepareNoProcess,
        action: HelperAction::CodexVersionProbeV1,
        authority: LaunchAuthority::PreparationOnly,
        helper_boundary: HelperBoundary::SeparatelySignedNestedHelperRequired,
        target_provenance: CollectionProvenance::CollectedNpmSchemaV2,
        containment_profile: ContainmentProfile::MacosRestrictedHelperV1,
        host_contract_schema_version: HOST_CONTRACT_SCHEMA_VERSION,
        collection_receipt_schema_version: COLLECTION_RECEIPT_SCHEMA_VERSION,
        request_id: value.request_id.into(),
        request_digest: value.request_digest.into(),
        binding_digest: value.binding_digest.into(),
        preparation_receipt_id: value.preparation_receipt_id.into(),
        preparation_receipt_digest: value.preparation_receipt_digest.into(),
        prepared_at: value.prepared_at.into(),
        session_id: value.session_id.into(),
        session_snapshot_digest: value.session_snapshot_digest.into(),
        workspace_digest: value.workspace_digest.into(),
        plan_id: value.plan_id.into(),
        plan_snapshot_digest: value.plan_snapshot_digest.into(),
        process_run_id: value.process_run_id.into(),
        process_run_spec_digest: value.process_run_spec_digest.into(),
        grant_id: value.grant_id.into(),
        grant_receipt_digest: value.grant_receipt_digest.into(),
        admission_id: value.admission_id.into(),
        admission_receipt_digest: value.admission_receipt_digest.into(),
        attempt_id: value.attempt_id.into(),
        probe_plan_digest: value.probe_plan_digest.into(),
        candidate_id: value.candidate_id.into(),
        launcher_chain_identity_digest: value.launcher_chain_identity_digest.into(),
        preflight_identity_digest: value.preflight_identity_digest.into(),
        containment_identity_digest: value.containment_identity_digest.into(),
        host_instance_identity_digest: value.host_instance_identity_digest.into(),
        boot_session_identity_digest: value.boot_session_identity_digest.into(),
        os_build_identity_digest: value.os_build_identity_digest.into(),
        helper_code_identity_digest: value.helper_code_identity_digest.into(),
        helper_entitlements_identity_digest: value.helper_entitlements_identity_digest.into(),
        enforcement_policy_identity_digest: value.enforcement_policy_identity_digest.into(),
        manual_opt_in_required: true,
        runnable: false,
        supported: false,
        process_start_enabled: false,
        execution_reserved: false,
        provider_traffic: ProviderTraffic::None,
        user_workspace_writes_enabled: false,
        frame_digest: String::new(),
    };
    request.frame_digest = request_frame_digest(&request);
    request.validate_shape()?;
    Ok(request)
}

pub fn prepare_shape_consistent_non_executing_response(
    request: &PreparationRequestFrame,
) -> Result<PreparationResponseFrame, ProtocolError> {
    request.validate_shape()?;
    let mut response = PreparationResponseFrame {
        protocol_version: PROTOCOL_VERSION,
        message_kind: PreparationMessageKind::PrepareNoProcess,
        action: HelperAction::CodexVersionProbeV1,
        request_frame_digest: request.frame_digest.clone(),
        request_id: request.request_id.clone(),
        request_digest: request.request_digest.clone(),
        binding_digest: request.binding_digest.clone(),
        preparation_receipt_id: request.preparation_receipt_id.clone(),
        preparation_receipt_digest: request.preparation_receipt_digest.clone(),
        attempt_id: request.attempt_id.clone(),
        state: PreparationResultState::ShapeConsistentNoProcess,
        host_authenticated: false,
        freshness_verified: false,
        launch_authority_verified: false,
        helper_invoked: false,
        process_started: false,
        execution_reserved: false,
        runnable: false,
        supported: false,
        provider_traffic: ProviderTraffic::None,
        user_workspace_writes_enabled: false,
        frame_digest: String::new(),
    };
    response.frame_digest = response_frame_digest(&response);
    response.validate_shape()?;
    Ok(response)
}

pub fn encode_preparation_request(
    value: &PreparationRequestFrame,
) -> Result<Vec<u8>, ProtocolError> {
    value.validate_shape()?;
    encode_frame(value)
}

pub fn decode_preparation_request(frame: &[u8]) -> Result<PreparationRequestFrame, ProtocolError> {
    let value: PreparationRequestFrame = decode_frame(frame)?;
    value.validate_shape()?;
    Ok(value)
}

pub fn encode_preparation_response(
    value: &PreparationResponseFrame,
) -> Result<Vec<u8>, ProtocolError> {
    value.validate_shape()?;
    encode_frame(value)
}

pub fn decode_preparation_response(
    frame: &[u8],
) -> Result<PreparationResponseFrame, ProtocolError> {
    let value: PreparationResponseFrame = decode_frame(frame)?;
    value.validate_shape()?;
    Ok(value)
}

impl PreparationRequestFrame {
    /// Validates canonical shape and internal transcript consistency only.
    ///
    /// This does not authenticate the host, prove freshness, or grant authority.
    pub fn validate_shape(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedProtocolVersion);
        }
        validate_identifiers(&[
            &self.request_id,
            &self.preparation_receipt_id,
            &self.session_id,
            &self.plan_id,
            &self.process_run_id,
            &self.grant_id,
            &self.admission_id,
            &self.attempt_id,
            &self.candidate_id,
        ])?;
        validate_digests(&[
            &self.request_digest,
            &self.binding_digest,
            &self.preparation_receipt_digest,
            &self.session_snapshot_digest,
            &self.workspace_digest,
            &self.plan_snapshot_digest,
            &self.process_run_spec_digest,
            &self.grant_receipt_digest,
            &self.admission_receipt_digest,
            &self.probe_plan_digest,
            &self.launcher_chain_identity_digest,
            &self.preflight_identity_digest,
            &self.containment_identity_digest,
            &self.host_instance_identity_digest,
            &self.boot_session_identity_digest,
            &self.os_build_identity_digest,
            &self.helper_code_identity_digest,
            &self.helper_entitlements_identity_digest,
            &self.enforcement_policy_identity_digest,
            &self.frame_digest,
        ])?;
        validate_prepared_at(&self.prepared_at)?;
        if self.message_kind != PreparationMessageKind::PrepareNoProcess
            || self.action != HelperAction::CodexVersionProbeV1
            || self.authority != LaunchAuthority::PreparationOnly
            || self.helper_boundary != HelperBoundary::SeparatelySignedNestedHelperRequired
            || self.target_provenance != CollectionProvenance::CollectedNpmSchemaV2
            || self.containment_profile != ContainmentProfile::MacosRestrictedHelperV1
            || self.provider_traffic != ProviderTraffic::None
            || self.host_contract_schema_version != HOST_CONTRACT_SCHEMA_VERSION
            || self.collection_receipt_schema_version != COLLECTION_RECEIPT_SCHEMA_VERSION
            || !self.manual_opt_in_required
            || self.runnable
            || self.supported
            || self.process_start_enabled
            || self.execution_reserved
            || self.user_workspace_writes_enabled
        {
            return Err(ProtocolError::FixedPolicyViolation);
        }
        if self.binding_digest != host_binding_digest(self)
            || self.request_id != expected_host_request_id(&self.binding_digest)
            || self.request_digest != host_request_digest(self)
            || self.preparation_receipt_id
                != expected_host_preparation_receipt_id(
                    &self.request_id,
                    &self.request_digest,
                    &self.prepared_at,
                )
            || self.preparation_receipt_digest != host_preparation_receipt_digest(self)
        {
            return Err(ProtocolError::HostTranscriptMismatch);
        }
        if self.frame_digest != request_frame_digest(self) {
            return Err(ProtocolError::FrameDigestMismatch);
        }
        Ok(())
    }
}

impl PreparationResponseFrame {
    /// Validates canonical shape and internal transcript consistency only.
    pub fn validate_shape(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedProtocolVersion);
        }
        validate_identifiers(&[
            &self.request_id,
            &self.preparation_receipt_id,
            &self.attempt_id,
        ])?;
        validate_digests(&[
            &self.request_frame_digest,
            &self.request_digest,
            &self.binding_digest,
            &self.preparation_receipt_digest,
            &self.frame_digest,
        ])?;
        if self.message_kind != PreparationMessageKind::PrepareNoProcess
            || self.action != HelperAction::CodexVersionProbeV1
            || self.state != PreparationResultState::ShapeConsistentNoProcess
            || self.provider_traffic != ProviderTraffic::None
            || self.host_authenticated
            || self.freshness_verified
            || self.launch_authority_verified
            || self.helper_invoked
            || self.process_started
            || self.execution_reserved
            || self.runnable
            || self.supported
            || self.user_workspace_writes_enabled
        {
            return Err(ProtocolError::FixedPolicyViolation);
        }
        if self.frame_digest != response_frame_digest(self) {
            return Err(ProtocolError::FrameDigestMismatch);
        }
        Ok(())
    }
}

fn request_frame_digest(value: &PreparationRequestFrame) -> String {
    let protocol_version = value.protocol_version.to_string();
    let host_schema = value.host_contract_schema_version.to_string();
    let collection_schema = value.collection_receipt_schema_version.to_string();
    let flags = bool_flags(&[
        value.manual_opt_in_required,
        value.runnable,
        value.supported,
        value.process_start_enabled,
        value.execution_reserved,
        value.user_workspace_writes_enabled,
    ]);
    bounded_digest(
        b"ai-switchboard-codex-probe-helper-preparation-request-frame-v1\0",
        &[
            protocol_version.as_str(),
            value.message_kind.digest_value(),
            value.action.digest_value(),
            value.authority.digest_value(),
            value.helper_boundary.digest_value(),
            value.target_provenance.digest_value(),
            value.containment_profile.digest_value(),
            host_schema.as_str(),
            collection_schema.as_str(),
            value.request_id.as_str(),
            value.request_digest.as_str(),
            value.binding_digest.as_str(),
            value.preparation_receipt_id.as_str(),
            value.preparation_receipt_digest.as_str(),
            value.prepared_at.as_str(),
            value.session_id.as_str(),
            value.session_snapshot_digest.as_str(),
            value.workspace_digest.as_str(),
            value.plan_id.as_str(),
            value.plan_snapshot_digest.as_str(),
            value.process_run_id.as_str(),
            value.process_run_spec_digest.as_str(),
            value.grant_id.as_str(),
            value.grant_receipt_digest.as_str(),
            value.admission_id.as_str(),
            value.admission_receipt_digest.as_str(),
            value.attempt_id.as_str(),
            value.probe_plan_digest.as_str(),
            value.candidate_id.as_str(),
            value.launcher_chain_identity_digest.as_str(),
            value.preflight_identity_digest.as_str(),
            value.containment_identity_digest.as_str(),
            value.host_instance_identity_digest.as_str(),
            value.boot_session_identity_digest.as_str(),
            value.os_build_identity_digest.as_str(),
            value.helper_code_identity_digest.as_str(),
            value.helper_entitlements_identity_digest.as_str(),
            value.enforcement_policy_identity_digest.as_str(),
            flags.as_str(),
            value.provider_traffic.digest_value(),
        ],
    )
}

fn response_frame_digest(value: &PreparationResponseFrame) -> String {
    let protocol_version = value.protocol_version.to_string();
    let flags = bool_flags(&[
        value.host_authenticated,
        value.freshness_verified,
        value.launch_authority_verified,
        value.helper_invoked,
        value.process_started,
        value.execution_reserved,
        value.runnable,
        value.supported,
        value.user_workspace_writes_enabled,
    ]);
    bounded_digest(
        b"ai-switchboard-codex-probe-helper-preparation-response-frame-v1\0",
        &[
            protocol_version.as_str(),
            value.message_kind.digest_value(),
            value.action.digest_value(),
            value.request_frame_digest.as_str(),
            value.request_id.as_str(),
            value.request_digest.as_str(),
            value.binding_digest.as_str(),
            value.preparation_receipt_id.as_str(),
            value.preparation_receipt_digest.as_str(),
            value.attempt_id.as_str(),
            value.state.digest_value(),
            flags.as_str(),
            value.provider_traffic.digest_value(),
        ],
    )
}

fn host_binding_digest(value: &PreparationRequestFrame) -> String {
    let collection_schema = value.collection_receipt_schema_version.to_string();
    bounded_digest(
        b"ai-switchboard-codex-helper-launch-binding-v1\0",
        &[
            value.session_id.as_str(),
            value.session_snapshot_digest.as_str(),
            value.workspace_digest.as_str(),
            value.plan_id.as_str(),
            value.plan_snapshot_digest.as_str(),
            value.process_run_id.as_str(),
            value.process_run_spec_digest.as_str(),
            value.grant_id.as_str(),
            value.grant_receipt_digest.as_str(),
            value.admission_id.as_str(),
            value.admission_receipt_digest.as_str(),
            value.attempt_id.as_str(),
            value.host_instance_identity_digest.as_str(),
            value.boot_session_identity_digest.as_str(),
            value.os_build_identity_digest.as_str(),
            value.containment_profile.host_value(),
            value.helper_code_identity_digest.as_str(),
            value.helper_entitlements_identity_digest.as_str(),
            value.enforcement_policy_identity_digest.as_str(),
            value.action.host_value(),
            value.helper_boundary.host_value(),
            value.target_provenance.host_value(),
            collection_schema.as_str(),
            value.probe_plan_digest.as_str(),
            value.candidate_id.as_str(),
            value.launcher_chain_identity_digest.as_str(),
            value.containment_identity_digest.as_str(),
            value.preflight_identity_digest.as_str(),
        ],
    )
}

fn expected_host_request_id(binding_digest: &str) -> String {
    format!(
        "codex-helper-request:{}",
        binding_digest.trim_start_matches("sha256:")
    )
}

fn host_request_digest(value: &PreparationRequestFrame) -> String {
    let schema = value.host_contract_schema_version.to_string();
    let flags = bool_flags(&[
        value.manual_opt_in_required,
        value.runnable,
        value.supported,
        value.process_start_enabled,
        value.user_workspace_writes_enabled,
    ]);
    bounded_digest(
        b"ai-switchboard-codex-helper-launch-request-v1\0",
        &[
            schema.as_str(),
            value.request_id.as_str(),
            value.binding_digest.as_str(),
            "prepared_no_process",
            flags.as_str(),
            value.provider_traffic.digest_value(),
        ],
    )
}

fn expected_host_preparation_receipt_id(
    request_id: &str,
    request_digest: &str,
    prepared_at: &str,
) -> String {
    let identity = bounded_digest(
        b"ai-switchboard-codex-helper-launch-preparation-receipt-id-v1\0",
        &[request_id, request_digest, prepared_at],
    );
    format!(
        "codex-helper-receipt:{}",
        identity.trim_start_matches("sha256:")
    )
}

fn host_preparation_receipt_digest(value: &PreparationRequestFrame) -> String {
    let schema = value.host_contract_schema_version.to_string();
    bounded_digest(
        b"ai-switchboard-codex-helper-launch-preparation-receipt-v1\0",
        &[
            schema.as_str(),
            value.preparation_receipt_id.as_str(),
            value.request_id.as_str(),
            value.request_digest.as_str(),
            value.prepared_at.as_str(),
            "validated_no_process",
            "000000",
            value.provider_traffic.digest_value(),
        ],
    )
}

fn validate_identifiers(values: &[&str]) -> Result<(), ProtocolError> {
    if values.iter().any(|value| {
        value.is_empty()
            || value.len() > MAX_IDENTIFIER_BYTES
            || value.trim() != *value
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
    }) {
        return Err(ProtocolError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_digests(values: &[&str]) -> Result<(), ProtocolError> {
    if values.iter().any(|value| !is_lowercase_sha256(value)) {
        return Err(ProtocolError::InvalidDigest);
    }
    Ok(())
}

fn validate_prepared_at(value: &str) -> Result<(), ProtocolError> {
    let bytes = value.as_bytes();
    if !(20..=64).contains(&bytes.len())
        || !value.is_ascii()
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(ProtocolError::InvalidPreparedAt);
    }
    Ok(())
}

fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let payload = serde_json::to_vec(value).map_err(|_| ProtocolError::JsonRejected)?;
    if payload.is_empty() || payload.len() > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::EncodedFrameTooLarge);
    }
    let mut frame = Vec::with_capacity(FRAME_PREFIX_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

fn decode_frame<T: DeserializeOwned + Serialize>(frame: &[u8]) -> Result<T, ProtocolError> {
    if frame.len() < FRAME_PREFIX_BYTES {
        return Err(ProtocolError::FrameTooShort);
    }
    let payload_length = u32::from_be_bytes(
        frame[..FRAME_PREFIX_BYTES]
            .try_into()
            .expect("fixed prefix length"),
    ) as usize;
    if payload_length == 0 {
        return Err(ProtocolError::FrameLengthZero);
    }
    if payload_length > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::FrameLengthTooLarge);
    }
    if frame.len() != FRAME_PREFIX_BYTES + payload_length {
        return Err(ProtocolError::FrameLengthMismatch);
    }
    let payload = &frame[FRAME_PREFIX_BYTES..];
    let json = std::str::from_utf8(payload).map_err(|_| ProtocolError::InvalidUtf8)?;
    let value: T = serde_json::from_str(json).map_err(|_| ProtocolError::JsonRejected)?;
    let canonical = serde_json::to_vec(&value).map_err(|_| ProtocolError::JsonRejected)?;
    if canonical != payload {
        return Err(ProtocolError::NonCanonicalJson);
    }
    Ok(value)
}
