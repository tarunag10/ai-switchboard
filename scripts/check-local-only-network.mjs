import fs from "node:fs";

const docsPath = "docs/remote-destinations.md";
const packagePath = "package.json";

const requiredRegistrySignals = [
  "## Local-Only Boundary",
  "AI Switchboard does not include a remote account, billing, checkout, or paid",
  "Tauri updater feeds",
  "Sentry diagnostics",
  "Aptabase analytics",
  "Microsoft Clarity",
  "Support and external links",
  "Provider traffic is not an app-owned analytics or account destination",
  "Add a local-only guard or explain why the call is strictly user-initiated.",
];

const requiredGuardSignals = {
  "src/lib/localMode.ts": [
    "VITE_HEADROOM_LOCAL_ONLY",
    "VITE_HEADROOM_REMOTE_SERVICES",
    "VITE_HEADROOM_REMOTE_TELEMETRY",
    "return !localOnlyModeEnabled() && truthy(import.meta.env.VITE_HEADROOM_REMOTE_TELEMETRY)",
  ],
  "src/lib/analytics.ts": [
    "remoteTelemetryEnabled()",
    "invoke(\"track_analytics_event\"",
  ],
  "src/lib/bootstrapSentry.ts": [
    "remoteTelemetryEnabled()",
    "Sentry.captureException(error)",
  ],
  "src-tauri/src/analytics.rs": [
    "if local_mode::enabled()",
    "return None;",
    "https://eu.aptabase.com/api/v0/events",
    "https://us.aptabase.com/api/v0/events",
  ],
  "src-tauri/src/pricing.rs": [
    "const LOCAL_ONLY_REMOTE_SERVICES_ERROR",
    "reject_remote_services_in_local_only()?",
    "Remote account and billing services are disabled in local-only mode.",
  ],
  "src-tauri/src/lib.rs": [
    "reject_contact_request_in_local_only()?",
    "Support/contact requests are disabled in local-only mode.",
  ],
  "src-tauri/src/external_open.rs": [
    "validate_external_link_url",
  ],
};

const documentedAppOwnedRemoteSignals = [
  {
    id: "aptabase",
    sourcePath: "src-tauri/src/analytics.rs",
    needles: [
      "https://eu.aptabase.com/api/v0/events",
      "https://us.aptabase.com/api/v0/events",
    ],
    docsNeedles: ["Aptabase analytics", "Disabled when local-only."],
  },
  {
    id: "support-links",
    sourcePath: "src/App.tsx",
    needles: ["https://github.com/tarunag10/mac-ai-switchboard/issues"],
    docsNeedles: ["Support and external links", "user-initiated links"],
  },
  {
    id: "updater",
    sourcePath: "docs/macos-release.md",
    needles: ["HEADROOM_UPDATER_ENDPOINTS"],
    docsNeedles: ["Tauri updater feeds", "Do not configure updater endpoints for local-only builds."],
  },
  {
    id: "sentry",
    sourcePath: "src/lib/bootstrapSentry.ts",
    needles: ["@sentry/react"],
    docsNeedles: ["Sentry diagnostics", "Disabled when local-only or remote telemetry is disabled."],
  },
];

const documentedProviderTrafficSignals = [
  {
    id: "anthropic-provider-traffic",
    sourcePath: "src-tauri/src/proxy_intercept.rs",
    needles: ["https://api.anthropic.com"],
    docsNeedles: ["Provider Traffic", "Anthropic"],
  },
  {
    id: "openai-provider-traffic",
    sourcePath: "src-tauri/src/proxy_intercept.rs",
    needles: ["https://api.openai.com", "https://chatgpt.com/backend-api/wham/usage"],
    docsNeedles: ["Provider Traffic", "OpenAI"],
  },
];

const documentedToolDownloadSignals = [
  {
    id: "pinned-headroom-wheel",
    sourcePath: "src-tauri/src/tool_manager.rs",
    needles: ["https://files.pythonhosted.org/", "HEADROOM_PINNED_WHEEL_URL"],
    docsNeedles: ["Tool And Dependency Downloads", "Pinned `headroom-ai` wheel install."],
  },
  {
    id: "vendor-wheel-index",
    sourcePath: "src-tauri/src/tool_manager.rs",
    needles: [
      "https://github.com/gglucass/headroom-desktop/releases/expanded_assets/vendor-wheels-v1",
    ],
    docsNeedles: ["Vendor wheel index", "Pinned by the desktop release."],
  },
];

const forbiddenExecutableRemoteFragments = [
  {
    fragment: "buy.polar.sh",
    allowedPaths: ["src-tauri/src/pricing.rs"],
    requiredContext: "local-only blocks",
  },
  {
    fragment: "app.aptabase.com",
    allowedPaths: [],
    requiredContext: "use regional Aptabase ingest URLs only",
  },
  {
    fragment: "clarity.ms",
    allowedPaths: [],
    requiredContext: "frontend Clarity must remain env-gated and absent from local-free bundles",
  },
  {
    fragment: "api.headroom",
    allowedPaths: [],
    requiredContext: "no paid or upstream Headroom account API is allowed",
  },
];

const scanPaths = [
  "src",
  "src-tauri/src",
  "src-tauri/tauri.conf.json",
  "package.json",
];

const failures = [];

function read(path) {
  return fs.readFileSync(path, "utf8");
}

function requireFile(path) {
  if (!fs.existsSync(path)) {
    failures.push(`Missing ${path}`);
    return false;
  }
  return true;
}

function walk(path, files = []) {
  if (!fs.existsSync(path)) {
    return files;
  }
  const stat = fs.statSync(path);
  if (stat.isDirectory()) {
    for (const entry of fs.readdirSync(path)) {
      if (["node_modules", "target", "dist", "coverage"].includes(entry)) {
        continue;
      }
      walk(`${path}/${entry}`, files);
    }
    return files;
  }
  if (stat.isFile()) {
    files.push(path);
  }
  return files;
}

if (requireFile(docsPath)) {
  const docs = read(docsPath);
  for (const signal of requiredRegistrySignals) {
    if (!docs.includes(signal)) {
      failures.push(`${docsPath} missing local-only registry signal: ${signal}`);
    }
  }

  for (const item of documentedAppOwnedRemoteSignals) {
    if (!requireFile(item.sourcePath)) {
      continue;
    }
    const source = read(item.sourcePath);
    const sourceHasNeedle = item.needles.some((needle) => source.includes(needle));
    if (!sourceHasNeedle) {
      failures.push(`${item.sourcePath} missing expected ${item.id} source signal`);
    }
    for (const docsNeedle of item.docsNeedles) {
      if (!docs.includes(docsNeedle)) {
        failures.push(`${docsPath} missing ${item.id} documentation signal: ${docsNeedle}`);
      }
    }
  }

  for (const item of documentedProviderTrafficSignals) {
    if (!requireFile(item.sourcePath)) {
      continue;
    }
    const source = read(item.sourcePath);
    const sourceHasNeedle = item.needles.some((needle) => source.includes(needle));
    if (!sourceHasNeedle) {
      failures.push(`${item.sourcePath} missing expected ${item.id} provider signal`);
    }
    for (const docsNeedle of item.docsNeedles) {
      if (!docs.includes(docsNeedle)) {
        failures.push(`${docsPath} missing ${item.id} documentation signal: ${docsNeedle}`);
      }
    }
  }

  for (const item of documentedToolDownloadSignals) {
    if (!requireFile(item.sourcePath)) {
      continue;
    }
    const source = read(item.sourcePath);
    const sourceHasNeedle = item.needles.some((needle) => source.includes(needle));
    if (!sourceHasNeedle) {
      failures.push(`${item.sourcePath} missing expected ${item.id} download signal`);
    }
    for (const docsNeedle of item.docsNeedles) {
      if (!docs.includes(docsNeedle)) {
        failures.push(`${docsPath} missing ${item.id} documentation signal: ${docsNeedle}`);
      }
    }
  }
}

for (const [path, signals] of Object.entries(requiredGuardSignals)) {
  if (!requireFile(path)) {
    continue;
  }
  const body = read(path);
  for (const signal of signals) {
    if (!body.includes(signal)) {
      failures.push(`${path} missing local-only network guard signal: ${signal}`);
    }
  }
}

if (requireFile(packagePath)) {
  const body = read(packagePath);
  if (!body.includes('"check:local-only-network"')) {
    failures.push(`${packagePath} missing check:local-only-network script`);
  }
}

const scannedFiles = scanPaths.flatMap((path) => walk(path));
for (const file of scannedFiles) {
  const body = read(file);
  for (const rule of forbiddenExecutableRemoteFragments) {
    if (!body.includes(rule.fragment)) {
      continue;
    }
    if (rule.allowedPaths.includes(file) && body.includes(rule.requiredContext)) {
      continue;
    }
    failures.push(
      `${file} contains ${rule.fragment} without required local-only context: ${rule.requiredContext}`,
    );
  }
}

if (failures.length > 0) {
  console.error("Local-only network certification failed:");
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(
  `Local-only network certification passed for ${Object.keys(requiredGuardSignals).length} guard surfaces, ${documentedAppOwnedRemoteSignals.length} documented app-owned remote-service surfaces, ${documentedProviderTrafficSignals.length} documented provider-traffic surfaces, and ${documentedToolDownloadSignals.length} documented managed-download surfaces.`,
);
