//! Fail-closed, content-free multi-tenant policy contracts.
//!
//! This module deliberately accepts only validated opaque identifiers, enums,
//! and counters. Prompts, responses, credentials, endpoint URLs, and arbitrary
//! audit messages have no place in the type system.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct PolicyId(String);

impl PolicyId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, PolicyError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        if !valid {
            return Err(PolicyError::InvalidIdentifier);
        }
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for PolicyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TenantRole {
    Admin,
    User,
    Auditor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantScope {
    pub(crate) tenant_id: PolicyId,
    pub(crate) team_id: PolicyId,
    pub(crate) project_id: PolicyId,
    pub(crate) account_id: PolicyId,
    pub(crate) workspace_id: PolicyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantActor {
    pub(crate) actor_id: PolicyId,
    pub(crate) scope: TenantScope,
    pub(crate) role: TenantRole,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskClass {
    Formatting,
    CodeGeneration,
    CodeReview,
    RepositoryAnalysis,
    Administrative,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EndpointLocation {
    Local,
    OrganizationManaged,
    External,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuotaLimit {
    pub(crate) requests: u64,
    pub(crate) input_tokens: u64,
    pub(crate) cost_microunits: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuotaUsage {
    pub(crate) requests: u64,
    pub(crate) input_tokens: u64,
    pub(crate) cost_microunits: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayeredUsage {
    pub(crate) tenant: QuotaUsage,
    pub(crate) team: QuotaUsage,
    pub(crate) project: QuotaUsage,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoutingPolicy {
    pub(crate) allowed_task_classes: BTreeSet<TaskClass>,
    pub(crate) automatic_routing_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrivacyPolicy {
    pub(crate) maximum_data_classification: DataClassification,
    pub(crate) allow_external_endpoints: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheIsolationPolicy {
    pub(crate) enabled: bool,
    pub(crate) require_project_namespace: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantPolicyBundle {
    pub(crate) tenant_id: PolicyId,
    pub(crate) tenant_quota: QuotaLimit,
    pub(crate) team_quotas: BTreeMap<PolicyId, QuotaLimit>,
    pub(crate) project_quotas: BTreeMap<PolicyId, QuotaLimit>,
    pub(crate) routing: RoutingPolicy,
    pub(crate) privacy: PrivacyPolicy,
    pub(crate) cache: CacheIsolationPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointEntitlement {
    pub(crate) endpoint_id: PolicyId,
    pub(crate) tenant_id: PolicyId,
    pub(crate) allowed_teams: BTreeSet<PolicyId>,
    pub(crate) allowed_projects: BTreeSet<PolicyId>,
    pub(crate) allowed_accounts: BTreeSet<PolicyId>,
    pub(crate) allowed_workspaces: BTreeSet<PolicyId>,
    pub(crate) allowed_task_classes: BTreeSet<TaskClass>,
    pub(crate) location: EndpointLocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheNamespaceClaim {
    pub(crate) tenant_id: PolicyId,
    pub(crate) team_id: PolicyId,
    pub(crate) project_id: PolicyId,
    pub(crate) account_id: PolicyId,
    pub(crate) workspace_id: PolicyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantRouteRequest {
    pub(crate) request_id: PolicyId,
    pub(crate) occurred_at_unix_ms: u64,
    pub(crate) actor: TenantActor,
    pub(crate) endpoint_id: PolicyId,
    pub(crate) task_class: TaskClass,
    pub(crate) data_classification: DataClassification,
    pub(crate) automatic_route: bool,
    pub(crate) cache_requested: bool,
    pub(crate) cache_namespace: Option<CacheNamespaceClaim>,
    pub(crate) current_usage: LayeredUsage,
    pub(crate) charge: QuotaUsage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditAction {
    RouteRequest,
    PolicyUpdate,
    AuditRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PolicyReason {
    Allowed,
    InvalidPolicy,
    CrossTenantDenied,
    RoleDenied,
    TeamQuotaMissing,
    ProjectQuotaMissing,
    TenantQuotaExceeded,
    TeamQuotaExceeded,
    ProjectQuotaExceeded,
    TaskClassDenied,
    AutomaticRoutingDenied,
    EndpointNotEntitled,
    EndpointTeamDenied,
    EndpointProjectDenied,
    EndpointAccountDenied,
    EndpointWorkspaceDenied,
    ExternalEndpointDenied,
    PrivacyClassificationDenied,
    CacheDenied,
    CacheNamespaceMissing,
    CacheNamespaceMismatch,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantAuditRecord {
    pub(crate) request_id: PolicyId,
    pub(crate) occurred_at_unix_ms: u64,
    pub(crate) tenant_id: PolicyId,
    pub(crate) team_id: PolicyId,
    pub(crate) project_id: PolicyId,
    pub(crate) account_id: PolicyId,
    pub(crate) workspace_id: PolicyId,
    pub(crate) actor_id: PolicyId,
    pub(crate) actor_role: TenantRole,
    pub(crate) action: AuditAction,
    pub(crate) endpoint_id: Option<PolicyId>,
    pub(crate) task_class: Option<TaskClass>,
    pub(crate) allowed: bool,
    pub(crate) reason: PolicyReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantPolicyDecision {
    pub(crate) allowed: bool,
    pub(crate) reason: PolicyReason,
    pub(crate) audit: TenantAuditRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PolicyError {
    InvalidIdentifier,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentifier => formatter.write_str("invalid opaque policy identifier"),
        }
    }
}

pub(crate) fn evaluate_tenant_route(
    policy: &TenantPolicyBundle,
    entitlements: &[EndpointEntitlement],
    request: &TenantRouteRequest,
) -> TenantPolicyDecision {
    let deny = |reason| route_decision(request, false, reason);
    if request.actor.scope.tenant_id != policy.tenant_id {
        return deny(PolicyReason::CrossTenantDenied);
    }
    if request.actor.role == TenantRole::Auditor {
        return deny(PolicyReason::RoleDenied);
    }
    if request.charge.requests == 0 {
        return deny(PolicyReason::InvalidPolicy);
    }
    let Some(team_limit) = policy.team_quotas.get(&request.actor.scope.team_id) else {
        return deny(PolicyReason::TeamQuotaMissing);
    };
    let Some(project_limit) = policy.project_quotas.get(&request.actor.scope.project_id) else {
        return deny(PolicyReason::ProjectQuotaMissing);
    };
    for (usage, limit, reason) in [
        (
            &request.current_usage.tenant,
            &policy.tenant_quota,
            PolicyReason::TenantQuotaExceeded,
        ),
        (
            &request.current_usage.team,
            team_limit,
            PolicyReason::TeamQuotaExceeded,
        ),
        (
            &request.current_usage.project,
            project_limit,
            PolicyReason::ProjectQuotaExceeded,
        ),
    ] {
        match quota_allows(usage, &request.charge, limit) {
            Ok(true) => {}
            Ok(false) => return deny(reason),
            Err(()) => return deny(PolicyReason::ArithmeticOverflow),
        }
    }
    if !policy
        .routing
        .allowed_task_classes
        .contains(&request.task_class)
    {
        return deny(PolicyReason::TaskClassDenied);
    }
    if request.automatic_route && !policy.routing.automatic_routing_enabled {
        return deny(PolicyReason::AutomaticRoutingDenied);
    }
    if request.data_classification > policy.privacy.maximum_data_classification {
        return deny(PolicyReason::PrivacyClassificationDenied);
    }

    let matching_endpoint: Vec<_> = entitlements
        .iter()
        .filter(|entitlement| entitlement.endpoint_id == request.endpoint_id)
        .collect();
    if matching_endpoint
        .iter()
        .any(|entitlement| entitlement.tenant_id != request.actor.scope.tenant_id)
        && !matching_endpoint
            .iter()
            .any(|entitlement| entitlement.tenant_id == request.actor.scope.tenant_id)
    {
        return deny(PolicyReason::CrossTenantDenied);
    }
    if matching_endpoint
        .iter()
        .filter(|entitlement| entitlement.tenant_id == request.actor.scope.tenant_id)
        .count()
        > 1
    {
        return deny(PolicyReason::InvalidPolicy);
    }
    let Some(entitlement) = matching_endpoint
        .into_iter()
        .find(|entitlement| entitlement.tenant_id == request.actor.scope.tenant_id)
    else {
        return deny(PolicyReason::EndpointNotEntitled);
    };
    if !entitlement
        .allowed_teams
        .contains(&request.actor.scope.team_id)
    {
        return deny(PolicyReason::EndpointTeamDenied);
    }
    if !entitlement
        .allowed_projects
        .contains(&request.actor.scope.project_id)
    {
        return deny(PolicyReason::EndpointProjectDenied);
    }
    if !entitlement
        .allowed_accounts
        .contains(&request.actor.scope.account_id)
    {
        return deny(PolicyReason::EndpointAccountDenied);
    }
    if !entitlement
        .allowed_workspaces
        .contains(&request.actor.scope.workspace_id)
    {
        return deny(PolicyReason::EndpointWorkspaceDenied);
    }
    if !entitlement
        .allowed_task_classes
        .contains(&request.task_class)
    {
        return deny(PolicyReason::TaskClassDenied);
    }
    if entitlement.location == EndpointLocation::External
        && !policy.privacy.allow_external_endpoints
    {
        return deny(PolicyReason::ExternalEndpointDenied);
    }
    if request.cache_requested {
        if !policy.cache.enabled {
            return deny(PolicyReason::CacheDenied);
        }
        let Some(namespace) = request.cache_namespace.as_ref() else {
            return deny(PolicyReason::CacheNamespaceMissing);
        };
        let same_project = namespace.tenant_id == request.actor.scope.tenant_id
            && namespace.team_id == request.actor.scope.team_id
            && namespace.project_id == request.actor.scope.project_id
            && namespace.account_id == request.actor.scope.account_id
            && namespace.workspace_id == request.actor.scope.workspace_id;
        let same_team = namespace.tenant_id == request.actor.scope.tenant_id
            && namespace.team_id == request.actor.scope.team_id
            && namespace.account_id == request.actor.scope.account_id
            && namespace.workspace_id == request.actor.scope.workspace_id;
        if (policy.cache.require_project_namespace && !same_project)
            || (!policy.cache.require_project_namespace && !same_team)
        {
            return deny(PolicyReason::CacheNamespaceMismatch);
        }
    }
    route_decision(request, true, PolicyReason::Allowed)
}

pub(crate) fn authorize_policy_update(
    request_id: PolicyId,
    occurred_at_unix_ms: u64,
    actor: &TenantActor,
    policy_tenant_id: &PolicyId,
) -> TenantPolicyDecision {
    authorize_control_action(
        request_id,
        occurred_at_unix_ms,
        actor,
        policy_tenant_id,
        AuditAction::PolicyUpdate,
    )
}

pub(crate) fn authorize_audit_read(
    request_id: PolicyId,
    occurred_at_unix_ms: u64,
    actor: &TenantActor,
    audit_tenant_id: &PolicyId,
) -> TenantPolicyDecision {
    if &actor.scope.tenant_id != audit_tenant_id {
        return control_decision(
            request_id,
            occurred_at_unix_ms,
            actor,
            AuditAction::AuditRead,
            false,
            PolicyReason::CrossTenantDenied,
        );
    }
    let allowed = matches!(actor.role, TenantRole::Admin | TenantRole::Auditor);
    control_decision(
        request_id,
        occurred_at_unix_ms,
        actor,
        AuditAction::AuditRead,
        allowed,
        if allowed {
            PolicyReason::Allowed
        } else {
            PolicyReason::RoleDenied
        },
    )
}

fn authorize_control_action(
    request_id: PolicyId,
    occurred_at_unix_ms: u64,
    actor: &TenantActor,
    target_tenant_id: &PolicyId,
    action: AuditAction,
) -> TenantPolicyDecision {
    if &actor.scope.tenant_id != target_tenant_id {
        return control_decision(
            request_id,
            occurred_at_unix_ms,
            actor,
            action,
            false,
            PolicyReason::CrossTenantDenied,
        );
    }
    let allowed = actor.role == TenantRole::Admin;
    control_decision(
        request_id,
        occurred_at_unix_ms,
        actor,
        action,
        allowed,
        if allowed {
            PolicyReason::Allowed
        } else {
            PolicyReason::RoleDenied
        },
    )
}

fn quota_allows(usage: &QuotaUsage, charge: &QuotaUsage, limit: &QuotaLimit) -> Result<bool, ()> {
    let requests = usage.requests.checked_add(charge.requests).ok_or(())?;
    let input_tokens = usage
        .input_tokens
        .checked_add(charge.input_tokens)
        .ok_or(())?;
    let cost_microunits = usage
        .cost_microunits
        .checked_add(charge.cost_microunits)
        .ok_or(())?;
    Ok(requests <= limit.requests
        && input_tokens <= limit.input_tokens
        && cost_microunits <= limit.cost_microunits)
}

fn route_decision(
    request: &TenantRouteRequest,
    allowed: bool,
    reason: PolicyReason,
) -> TenantPolicyDecision {
    TenantPolicyDecision {
        allowed,
        reason,
        audit: TenantAuditRecord {
            request_id: request.request_id.clone(),
            occurred_at_unix_ms: request.occurred_at_unix_ms,
            tenant_id: request.actor.scope.tenant_id.clone(),
            team_id: request.actor.scope.team_id.clone(),
            project_id: request.actor.scope.project_id.clone(),
            account_id: request.actor.scope.account_id.clone(),
            workspace_id: request.actor.scope.workspace_id.clone(),
            actor_id: request.actor.actor_id.clone(),
            actor_role: request.actor.role,
            action: AuditAction::RouteRequest,
            endpoint_id: Some(request.endpoint_id.clone()),
            task_class: Some(request.task_class),
            allowed,
            reason,
        },
    }
}

fn control_decision(
    request_id: PolicyId,
    occurred_at_unix_ms: u64,
    actor: &TenantActor,
    action: AuditAction,
    allowed: bool,
    reason: PolicyReason,
) -> TenantPolicyDecision {
    TenantPolicyDecision {
        allowed,
        reason,
        audit: TenantAuditRecord {
            request_id,
            occurred_at_unix_ms,
            tenant_id: actor.scope.tenant_id.clone(),
            team_id: actor.scope.team_id.clone(),
            project_id: actor.scope.project_id.clone(),
            account_id: actor.scope.account_id.clone(),
            workspace_id: actor.scope.workspace_id.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.role,
            action,
            endpoint_id: None,
            task_class: None,
            allowed,
            reason,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> PolicyId {
        PolicyId::new(value).unwrap()
    }
    fn scope(tenant: &str) -> TenantScope {
        TenantScope {
            tenant_id: id(tenant),
            team_id: id("team-a"),
            project_id: id("project-a"),
            account_id: id("account-a"),
            workspace_id: id("workspace-a"),
        }
    }
    fn usage(value: u64) -> LayeredUsage {
        let layer = QuotaUsage {
            requests: value,
            input_tokens: value,
            cost_microunits: value,
        };
        LayeredUsage {
            tenant: layer.clone(),
            team: layer.clone(),
            project: layer,
        }
    }
    fn policy() -> TenantPolicyBundle {
        TenantPolicyBundle {
            tenant_id: id("tenant-a"),
            tenant_quota: QuotaLimit {
                requests: 100,
                input_tokens: 100,
                cost_microunits: 100,
            },
            team_quotas: BTreeMap::from([(
                id("team-a"),
                QuotaLimit {
                    requests: 50,
                    input_tokens: 50,
                    cost_microunits: 50,
                },
            )]),
            project_quotas: BTreeMap::from([(
                id("project-a"),
                QuotaLimit {
                    requests: 25,
                    input_tokens: 25,
                    cost_microunits: 25,
                },
            )]),
            routing: RoutingPolicy {
                allowed_task_classes: BTreeSet::from([TaskClass::Formatting]),
                automatic_routing_enabled: false,
            },
            privacy: PrivacyPolicy {
                maximum_data_classification: DataClassification::Confidential,
                allow_external_endpoints: false,
            },
            cache: CacheIsolationPolicy {
                enabled: true,
                require_project_namespace: true,
            },
        }
    }
    fn entitlement(tenant: &str) -> EndpointEntitlement {
        EndpointEntitlement {
            endpoint_id: id("endpoint-a"),
            tenant_id: id(tenant),
            allowed_teams: BTreeSet::from([id("team-a")]),
            allowed_projects: BTreeSet::from([id("project-a")]),
            allowed_accounts: BTreeSet::from([id("account-a")]),
            allowed_workspaces: BTreeSet::from([id("workspace-a")]),
            allowed_task_classes: BTreeSet::from([TaskClass::Formatting]),
            location: EndpointLocation::Local,
        }
    }
    fn request(tenant: &str) -> TenantRouteRequest {
        let actor_scope = scope(tenant);
        TenantRouteRequest {
            request_id: id("request-a"),
            occurred_at_unix_ms: 1_700_000_000_000,
            actor: TenantActor {
                actor_id: id("actor-a"),
                scope: actor_scope.clone(),
                role: TenantRole::User,
            },
            endpoint_id: id("endpoint-a"),
            task_class: TaskClass::Formatting,
            data_classification: DataClassification::Internal,
            automatic_route: false,
            cache_requested: true,
            cache_namespace: Some(CacheNamespaceClaim {
                tenant_id: actor_scope.tenant_id,
                team_id: actor_scope.team_id,
                project_id: actor_scope.project_id,
                account_id: actor_scope.account_id,
                workspace_id: actor_scope.workspace_id,
            }),
            current_usage: usage(1),
            charge: QuotaUsage {
                requests: 1,
                input_tokens: 1,
                cost_microunits: 1,
            },
        }
    }

    #[test]
    fn allows_only_fully_scoped_entitled_request() {
        let decision =
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &request("tenant-a"));
        assert!(decision.allowed);
        assert_eq!(decision.audit.reason, PolicyReason::Allowed);
    }

    #[test]
    fn explicitly_denies_cross_tenant_policy_endpoint_and_cache_access() {
        let decision =
            evaluate_tenant_route(&policy(), &[entitlement("tenant-b")], &request("tenant-b"));
        assert!(!decision.allowed);
        assert_eq!(decision.reason, PolicyReason::CrossTenantDenied);

        let endpoint_crossing =
            evaluate_tenant_route(&policy(), &[entitlement("tenant-b")], &request("tenant-a"));
        assert_eq!(endpoint_crossing.reason, PolicyReason::CrossTenantDenied);

        let mut cache_crossing = request("tenant-a");
        cache_crossing.cache_namespace.as_mut().unwrap().tenant_id = id("tenant-b");
        let decision =
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &cache_crossing);
        assert_eq!(decision.reason, PolicyReason::CacheNamespaceMismatch);

        let mut workspace_crossing = request("tenant-a");
        workspace_crossing
            .cache_namespace
            .as_mut()
            .unwrap()
            .workspace_id = id("workspace-b");
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &workspace_crossing)
                .reason,
            PolicyReason::CacheNamespaceMismatch
        );
    }

    #[test]
    fn missing_scope_quota_and_overflow_fail_closed() {
        let mut missing = policy();
        missing.project_quotas.clear();
        assert_eq!(
            evaluate_tenant_route(&missing, &[entitlement("tenant-a")], &request("tenant-a"))
                .reason,
            PolicyReason::ProjectQuotaMissing
        );
        let mut overflow = request("tenant-a");
        overflow.current_usage.tenant.requests = u64::MAX;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &overflow).reason,
            PolicyReason::ArithmeticOverflow
        );
        let mut zero_charge = request("tenant-a");
        zero_charge.charge.requests = 0;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &zero_charge).reason,
            PolicyReason::InvalidPolicy
        );
        let mut over_project = request("tenant-a");
        over_project.current_usage.project.requests = 25;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &over_project).reason,
            PolicyReason::ProjectQuotaExceeded
        );
    }

    #[test]
    fn routing_privacy_cache_and_endpoint_gates_are_independent() {
        let mut automatic = request("tenant-a");
        automatic.automatic_route = true;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &automatic).reason,
            PolicyReason::AutomaticRoutingDenied
        );
        let mut restricted = request("tenant-a");
        restricted.data_classification = DataClassification::Restricted;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &restricted).reason,
            PolicyReason::PrivacyClassificationDenied
        );
        let mut no_cache_namespace = request("tenant-a");
        no_cache_namespace.cache_namespace = None;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &no_cache_namespace)
                .reason,
            PolicyReason::CacheNamespaceMissing
        );
        let mut external = entitlement("tenant-a");
        external.location = EndpointLocation::External;
        assert_eq!(
            evaluate_tenant_route(&policy(), &[external], &request("tenant-a")).reason,
            PolicyReason::ExternalEndpointDenied
        );
        let mut wrong_account = entitlement("tenant-a");
        wrong_account.allowed_accounts.clear();
        assert_eq!(
            evaluate_tenant_route(&policy(), &[wrong_account], &request("tenant-a")).reason,
            PolicyReason::EndpointAccountDenied
        );
    }

    #[test]
    fn admin_user_and_auditor_capabilities_are_separated() {
        let mut actor = request("tenant-a").actor;
        assert_eq!(
            authorize_policy_update(id("admin-a"), 1, &actor, &id("tenant-a")).reason,
            PolicyReason::RoleDenied
        );
        assert_eq!(
            authorize_audit_read(id("audit-a"), 2, &actor, &id("tenant-a")).reason,
            PolicyReason::RoleDenied
        );
        actor.role = TenantRole::Auditor;
        assert!(authorize_audit_read(id("audit-b"), 3, &actor, &id("tenant-a")).allowed);
        assert!(!authorize_policy_update(id("admin-b"), 4, &actor, &id("tenant-a")).allowed);
        actor.role = TenantRole::Admin;
        assert!(authorize_policy_update(id("admin-c"), 5, &actor, &id("tenant-a")).allowed);
        assert_eq!(
            authorize_audit_read(id("audit-cross"), 6, &actor, &id("tenant-b")).reason,
            PolicyReason::CrossTenantDenied
        );
    }

    #[test]
    fn audit_records_are_structurally_content_and_secret_free() {
        assert!(PolicyId::new("prompt text with spaces").is_err());
        assert!(PolicyId::new("https://endpoint.example/v1").is_err());
        assert!(serde_json::from_str::<PolicyId>(r#""tenant/a""#).is_err());
        let decision =
            evaluate_tenant_route(&policy(), &[entitlement("tenant-a")], &request("tenant-a"));
        let serialized = serde_json::to_string(&decision.audit).unwrap();
        assert!(!serialized.contains("prompt"));
        assert!(!serialized.contains("response"));
        assert!(!serialized.contains("credential"));
        assert!(!serialized.contains("url"));
    }
}
