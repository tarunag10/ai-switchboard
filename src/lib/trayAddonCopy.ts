import type { BootstrapProgress, RuntimeUpgradeProgress } from "./types";

export const localFirstReadinessSourceSignals = [
  "detectionEvidence",
  "Settings import/export",
  "Settings migration actions",
  "Copy settings export",
  "Apply safe preferences",
] as const;

export interface AddonCopy {
  whatItDoes: string;
  installing?: string;
  uninstalling?: string;
  installed?: string;
  uninstalled?: string;
  enabling?: string;
  disabling?: string;
  disabled?: string;
}

export const addonCopy: Record<string, AddonCopy> = {
  rtk: {
    whatItDoes:
      "RTK installs into the managed runtime, adds itself to the shell PATH, and enables the bash auto-rewrite hook. Agent shell commands route through RTK so noisy output is compacted before it spends tokens.",
    installing: "Downloading RTK and registering the bash hook...",
    uninstalling: "Removing RTK, its PATH entry, and the bash hook...",
    uninstalled:
      "RTK removed. Shell commands run normally, without output rewriting.",
    enabling: "Enabling RTK and registering the bash hook...",
    disabling: "Disabling RTK and removing the bash hook...",
    disabled:
      "RTK is off but still installed. Re-enable it later without re-downloading.",
  },
  markitdown: {
    whatItDoes:
      "MarkItDown installs into the managed Python runtime, registers a document Read hook, adds one Switchboard-owned Claude permission for its app-owned conversion shim, and uses a local /tmp/headroom-markitdown conversion cache. Documents can be converted to Markdown before an agent reads them, without installing anything system-wide.",
    installing: "Installing MarkItDown and registering the Read hook...",
    uninstalling: "Removing MarkItDown, its Read hook, permission, and local conversion cache...",
    uninstalled:
      "MarkItDown removed. Your agent reads documents in their original format again.",
    enabling: "Enabling MarkItDown...",
    disabling: "Disabling MarkItDown and removing its managed Read hook and permission...",
    disabled:
      "MarkItDown is off. Its managed Read hook and permission are removed; the package stays installed but no longer converts documents.",
  },
  caveman: {
    whatItDoes:
      "Caveman writes Switchboard-managed instruction blocks into Claude Code and Codex. It nudges agents toward terse output without hiding legal, safety, or debugging detail.",
    installing: "Writing Caveman guidance blocks...",
    uninstalling: "Removing Caveman guidance blocks...",
    installed:
      "Caveman installed. Pick scoped, aggressive, or Compact Chinese experimental mode any time.",
    uninstalled: "Caveman removed. Managed terse-output blocks were deleted.",
    enabling: "Enabling Caveman guidance...",
    disabling: "Disabling Caveman guidance...",
    disabled: "Caveman is off. Re-enable it later without recreating settings.",
  },
  ponytail: {
    whatItDoes:
      "Ponytail registers its marketplace plugin in Claude Code and/or Codex when those CLIs are on PATH. It nudges agents toward smaller, simpler edits and can run an over-engineering audit.",
    installing: "Registering Ponytail in available coding clients...",
    uninstalling: "Removing Ponytail from registered coding clients...",
    uninstalled:
      "Ponytail removed. Your agent writes code without the Ponytail nudge.",
    installed:
      "Ponytail installed. Run /ponytail-audit in an agent to scan this codebase for over-engineering.",
    enabling: "Enabling Ponytail...",
    disabling: "Disabling Ponytail...",
    disabled:
      "Ponytail is off. It stays installed but no longer nudges agents.",
  },
};

export const connectorSupportWarnings: Record<string, string> = {};

export const idleBootstrapProgress: BootstrapProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "Idle",
  message: "Installer has not started.",
  currentStepEtaSeconds: 0,
  overallPercent: 0,
};

export const idleRuntimeUpgradeProgress: RuntimeUpgradeProgress = {
  running: false,
  complete: false,
  failed: false,
  currentStep: "Idle",
  message: "",
  overallPercent: 0,
  fromVersion: null,
  toVersion: null,
};

export const MAX_UPGRADE_AUTO_RETRIES = 2;
