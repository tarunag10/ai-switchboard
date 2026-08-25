# ADR-0008: Response cache bodies remain plaintext at rest behind disclosure until encryption has a threat driver

- Status: Accepted
- Date: 2026-08-25

## Context

The Exact Response Cache (see ADR-0005) persists provider response bodies so
identical non-streaming requests can be replayed locally. Current behavior:

- Bodies are stored **plaintext** in a local SQLite database at
  `<app user-profile data dir>/semantic-cache.sqlite3` (WAL journaling,
  `secure_delete=ON`), alongside a small `semantic-cache.json` state file.
- The lookup key is a SHA-256 of the fully scoped namespace plus prompt;
  prompt text is never stored as a key or column.
- Isolation is by explicit namespace (`provider`, `model`, `account`,
  `workspace`, `policy`, `request_variant`). Namespaces that are not fully
  scoped refuse to cache, and requests with streaming, tool/MCP use,
  sensitive-data flags, high temperature, rapidly changing repo state, open
  tool calls, or a no-cache marker bypass caching entirely.
- The cache is opt-in and disabled by default. At-rest protection today is
  only the operating system's user-profile file permissions.

## Decision

We accept plaintext bodies at rest for now, with three standing controls
instead of premature encryption:

1. **Explicit disclosure** in product surfaces that enable the cache: bodies
   are stored locally on disk in plaintext.
2. **Storage-path documentation**: the database location above is documented
   so users and reviewers can inspect, back up, or delete it.
3. **Clear action with receipt**: a user-invokable clear command deletes all
   cached bodies and returns a receipt (entries cleared, restore unavailable).

Encryption at rest (for example SQLCipher or application-level AES with a
Keychain-held key) is the documented upgrade path. It is deliberately gated on
a real threat need — such as demonstrated same-user process exposure,
compliance requirements, or multi-account shared-host scenarios — because key
management adds failure modes (key loss, migration bugs) that would touch the
same data the cache promises to replay faithfully.

## Alternatives

- Encrypt bodies immediately with an app-level cipher and OS keychain key.
- Store only hashes and never persist bodies (removes the replay value).
- Move cached bodies into a Keychain-protected opaque store.
- Disable body persistence entirely until encryption ships.

## Consequences

Users get honest disclosure, an auditable on-disk location, and a reliable
clear action without new dependencies or key-management risk. Plaintext
bodies remain readable by any process running as the same macOS user; the
namespace isolation limits cross-context reuse but is not encryption. Any
future encryption work must preserve the existing namespace-isolation tests,
TTL/purge semantics, and clear-receipt contract while adding a migration path
for existing plaintext rows (or documenting their deletion).

## Reversal strategy

If a concrete threat or compliance need emerges, a superseding ADR enables
encryption at rest with a one-time re-encrypt-or-delete migration of the
existing database, keeping the disclosure surface (now describing ciphertext)
and the clear action unchanged. Returning to plaintext afterward would require
the same superseding-ADR treatment with migration and rollback evidence, so
the accepted-risk posture cannot silently regress into something weaker.
