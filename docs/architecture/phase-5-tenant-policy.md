# Phase 5 multi-tenant policy isolation

The Phase 5 policy boundary is a fail-closed authorization module. It receives
validated opaque identifiers, enumerated classifications/actions, and integer
usage counters. It has no fields for prompts, responses, credentials, API keys,
endpoint URLs, arbitrary log messages, or request bodies.

Every routed request is evaluated in this order:

1. tenant identity matches the policy tenant;
2. the actor role may route requests (`user` or `admin`, never `auditor`);
3. tenant, team, and project quota records all exist and the request charge can
   be added without overflow or exceeding a limit;
4. task class, automatic-routing, and privacy classification policies allow the
   request;
5. an unambiguous endpoint entitlement exists for the same tenant, team,
   project, account, workspace, and task class;
6. external endpoint use is permitted by privacy policy;
7. requested cache use is enabled and its namespace claim matches the tenant,
   team, project, account, and workspace isolation policy.

Missing quota records, ambiguous endpoint entitlements, arithmetic overflow,
zero-request accounting charges, missing cache namespace claims, and unknown
endpoint entitlements deny the request. An endpoint identifier entitled only to
another tenant returns the explicit `cross_tenant_denied` reason. There is no
fallback to another tenant, team, project, endpoint, policy, or cache namespace.

## Administration and audit separation

- `admin` may route requests, update policy, and read audit records for its own
  tenant.
- `user` may route requests but may not update policy or read the tenant audit
  stream.
- `auditor` may read audit records for its own tenant but may not route requests
  or update policy.
- all roles are denied policy or audit access across tenants before role grants
  are considered.

Every decision returns a structured `TenantAuditRecord` with a validated event
identifier, caller-supplied numeric timestamp, tenant/team/project,
account/workspace, and actor identifiers, role, enumerated action, optional endpoint/task identifiers,
allow/deny result, and enumerated reason. The schema intentionally cannot carry
content or secrets. Persistence, retention, export, and recovery remain the
responsibility of an organization-managed audit store; this module only creates
the safe record and authorization decision.

## Integration boundary

The isolated implementation is `src-tauri/src/tenant_policy.rs`. It does not
mutate coding-client configuration, endpoint registries, caches, telemetry, or
policy files. Callers must supply authenticated scope, trusted current usage,
the proposed charge, and tenant-owned entitlements. A later persistence layer
must preserve the same compound tenant/team/project/account/workspace keys and
must not deserialize unvalidated free-form identifiers into these contracts.
