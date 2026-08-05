export interface RtkTaskPreset {
  id: "test" | "build" | "grep" | "git-log";
  label: string;
  envBlock: string;
  commandHints: readonly string[];
}

export const RTK_TASK_PRESETS: readonly RtkTaskPreset[] = [
  {
    id: "test",
    label: "Test",
    envBlock: [
      "# RTK task preset: test",
      "export RTK_TASK_PRESET=test",
      "# Prefix test commands with rtk, for example:",
      "# rtk npm test",
      "# rtk vitest run",
      "# rtk pytest",
      "# rtk cargo test --manifest-path src-tauri/Cargo.toml",
    ].join("\n"),
    commandHints: ["npm test", "vitest", "pytest", "cargo test"],
  },
  {
    id: "build",
    label: "Build",
    envBlock: [
      "# RTK task preset: build",
      "export RTK_TASK_PRESET=build",
      "# Prefix build commands with rtk, for example:",
      "# rtk npm run build",
      "# rtk cargo build --manifest-path src-tauri/Cargo.toml",
      "# rtk tsc",
    ].join("\n"),
    commandHints: ["npm run build", "cargo build", "tsc"],
  },
  {
    id: "grep",
    label: "Grep",
    envBlock: [
      "# RTK task preset: grep",
      "export RTK_TASK_PRESET=grep",
      "# Prefix search commands with rtk, for example:",
      "# rtk rg pattern src",
      "# rtk grep -R pattern src",
    ].join("\n"),
    commandHints: ["rg", "grep", "rtk grep"],
  },
  {
    id: "git-log",
    label: "Git log",
    envBlock: [
      "# RTK task preset: git-log",
      "export RTK_TASK_PRESET=git-log",
      "# Prefix git history commands with rtk, for example:",
      "# rtk git log --oneline -20",
      "# rtk git diff --stat",
    ].join("\n"),
    commandHints: ["git log", "git diff", "rtk git log"],
  },
] as const;

export function getRtkTaskPreset(
  presetId: string,
): RtkTaskPreset | undefined {
  return RTK_TASK_PRESETS.find((preset) => preset.id === presetId);
}

export function buildRtkShellProfileSnippet(presetId: string): string | null {
  return getRtkTaskPreset(presetId)?.envBlock ?? null;
}
