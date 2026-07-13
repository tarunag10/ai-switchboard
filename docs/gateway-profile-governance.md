# Gateway profile governance

AI Switchboard keeps LiteLLM, Langfuse, Cloudflare AI Gateway, and Kong as
explicit add-on profiles. The profile registry is a read-only contract used by
the Add-ons view, readiness checks, and release evidence; it does not contain
credentials and does not contact a gateway.

Every profile must declare:

- local or remote traffic boundary and prompt/output visibility;
- whether provider routing can be changed and whether secrets are needed;
- supported clients, Doctor evidence, savings-evidence confidence, and setup
  guidance;
- rollback behavior and Off-mode cleanup boundaries.

Remote profiles must disclose that traffic leaves the local app. Local profiles
cannot claim to modify provider routing. Secret-bearing profiles must direct
users to secure storage or an environment outside the repository. Guided and
gated profiles are never treated as managed writes.

Run the release governance gate with:

```bash
npm run check:governance
```

The gate checks both public governance documentation and the TypeScript profile
registry. It is deliberately static and credential-free so it is safe in CI and
on Vercel. Runtime unit tests additionally exercise malformed profiles and
duplicate identifiers.
