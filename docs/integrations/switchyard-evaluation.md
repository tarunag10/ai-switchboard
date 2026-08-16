# Switchyard optional-value evaluation

Decision date: 2026-08-16. Outcome: qualifies as an optional external
interoperability tool, but is not added as a mandatory runtime or embedded
routing dependency in Phase 4.

## Pinned provenance

- Repository: `NVIDIA-NeMo/Switchyard`
- Reviewed commit: `9ad6744b734bc27dbf6fb07f79c8c7a3cb086d8d`
- Commit date: 2026-08-14
- Release checked: `v0.2.0` (published 2026-08-10)
- Primary source: [README at the reviewed commit](https://github.com/NVIDIA-NeMo/Switchyard/blob/9ad6744b734bc27dbf6fb07f79c8c7a3cb086d8d/README.md)

The reviewed upstream explicitly labels Switchyard pre-alpha experimental
software whose API and algorithms may change significantly before v1.0.

## Plan criteria

| Optional-value criterion | Evidence | Result |
|---|---|---|
| Required protocol translation | Translates OpenAI Chat, OpenAI Responses, and Anthropic Messages to provider-native formats. | Meets |
| Desired multi-backend router | Provides random, LLM-classifier, signal-driven stage, escalation, passthrough, and custom routing. | Meets, but overlaps Switchboard policy |
| Agent launcher compatibility not easily provided by Switchboard | Published launchers target Claude Code, Codex CLI, and OpenClaw. | Meets |

## Integration decision

Switchyard offers real optional value, particularly for protocol translation
and one-command agent launchers. It must not become the vLLM/SGLang/llama.cpp
path: those runtimes remain directly selectable through Switchboard's
`InferenceEndpoint` registry.

No dedicated managed profile is promoted in Phase 4 because:

- the upstream project is explicitly pre-alpha;
- its routing layer substantially overlaps Switchboard's benchmark-backed
  policy brain;
- Switchboard would otherwise need to reconcile two independent routing
  decisions and attribution trails;
- a user-run Switchyard server can already be enrolled through the generic
  OpenAI-compatible endpoint when passthrough interoperability is wanted.

Re-evaluate a dedicated profile only when a concrete workflow requires its
Anthropic/Responses translation or launcher lifecycle and has compatibility,
rollback, attribution, and failure-mode fixtures. Switchboard must still not
install, start, or rewrite Switchyard configuration without a separately
approved managed-lifecycle phase.
