# Changelog

## 2026-07-01

### Managed connector repair audit

- Fixed Doctor so installed managed connectors that are detected but still Direct now get a repairable issue instead of only a passive warning.
- Added a Codex-specific Doctor repair issue for the detected-but-unrouted state, including missing `OPENAI_BASE_URL` and provider-block setup.
- Added Mode Inspector repair actions for Direct managed connectors.
- Filled managed Zed connector metadata so connector validation covers config surfaces, manual workflow, and Off-mode cleanup wording.
- Validated with frontend, desktop, connector, build, and Doctor-repair smoke checks.

### Connector status documentation sync

- Updated connector support docs, install notes, architecture docs, and the compatibility matrix so Gemini CLI, OpenCode, Windsurf, and Zed AI are documented as managed where their lifecycle is now implemented.

### Backend connector metadata sync

- Updated backend connector readiness metadata for Gemini CLI, OpenCode, Windsurf, and Zed AI so Doctor, Settings, and Repo Intelligence fallback copy no longer inherit manual-era routing language.
