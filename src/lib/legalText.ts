export const termsOfUseTitle = "Mac AI Switchboard Terms of Use";

export const termsOfUseParagraphs = [
  "Mac AI Switchboard is a local desktop utility for managing AI tool routing, helper runtimes, shell-output compression, and related workflow automation on this Mac.",
  "You control when routing and add-ons are enabled. Off mode is intended to stop local proxy routing and managed automation until you turn those features back on.",
  "The app may edit local configuration files only for features you enable. Managed edits should be reversible, backed up where applicable, and visible through app diagnostics.",
  "Remote model providers, account services, pricing, update checks, telemetry, and support links are separate services unless explicitly labeled as Mac AI Switchboard-owned.",
];

export const privacyNoticeTitle = "Mac AI Switchboard Privacy Notice";

export const privacyNoticeParagraphs = [
  "Mac AI Switchboard stores its app state, diagnostics, routing mode, and managed configuration evidence locally on this Mac.",
  "When enabled, the app may read local shell, Claude, Codex, MCP, LaunchAgent, repository, and runtime files to detect setup health and build local diagnostics.",
  "Local-only mode should avoid account, pricing, telemetry, support, analytics, and update-network calls unless you explicitly enable a remote feature.",
  "Diagnostics and savings summaries can include local paths, command metadata, token counts, runtime status, and provider routing evidence. Review exported diagnostics before sharing them.",
  "Secrets and access tokens should remain in macOS Keychain or provider-owned tools. Mac AI Switchboard should not ask you to paste provider secrets into diagnostics.",
];
