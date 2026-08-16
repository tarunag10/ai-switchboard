# Experimental dsh context prototype

Status: experimental; no DeepSeek Harness core patch.

Upstream boundary observed 2026-08-16: the official [DeepSeek Harness repository](https://github.com/deepseek-ai/deepseek-harness) describes dsh as a developer preview, says its plugin architecture is iterating rapidly, and explicitly warns of compatibility-breaking changes. Consequently this prototype does not claim a stable upstream plugin version or directly import an upstream lifecycle API; `before_agent_run` is Switchboard's proposed boundary until a supported, versioned dsh seam is available.

The prototype maps the stable, read-only `ContextProvider` pack into a proposed `before_agent_run` dsh plugin payload. The payload contains the pack title and purpose, repository/index identities, selected repository paths, and ranking evidence. It does not read file contents or mutate the repository.

Each insertion records:

- source and inserted estimated tokens;
- savings versus the Repo Intelligence full-scan estimate;
- selected-file count;
- task-term match count;
- ranking-evidence count.

A deterministic SHA-256 replay identity is derived from the adapter version and stable pack fields. Session-local state suppresses a duplicate insertion of the same pack, while replaying the same pack preserves its identity. The prototype remains internal until dsh exposes a supported, versioned plugin lifecycle; it does not assume an upstream config schema or add a dsh dependency.
