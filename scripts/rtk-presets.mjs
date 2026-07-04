const FRAMEWORK_IDS = new Set(["vitest", "jest", "pytest", "cargo"]);

const re = (source, flags = "i") => new RegExp(source, flags);

export const frameworkPresets = [
  {
    id: "vitest",
    label: "Vitest",
    autoDetect: {
      commands: ["vitest", "vite test", "npm run test", "pnpm test", "yarn test"],
      filePatterns: ["vitest.config.", ".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx"],
      manifestSignals: ["vitest", "@vitest/coverage-v8", "vite"],
      outputPatterns: ["^\\s*(FAIL|Failed)\\s+", "^\\s*AssertionError", "^\\s*Test Files\\s+"],
    },
    preservePatterns: [
      "^\\s*(FAIL|Failed)\\s+",
      "^\\s*(Error|TypeError|ReferenceError|AssertionError|Expected|Received)\\b",
      "^\\s*[+-]\\s",
      "^\\s*>\\s*\\d+\\s*\\|",
      "\\S+\\.(test|spec)\\.(ts|tsx|js|jsx):\\d+:\\d+",
      "^\\s*(Test Files|Tests|Snapshots|Duration)\\s+",
      "^\\s*Caused by:",
      "^\\s*at\\s+",
    ],
    dropPatterns: [
      "^\\s*[✓✔]\\s+",
      "^\\s*stdout\\s*\\|",
      "^\\s*stderr\\s*\\|",
      "^\\s*coverage\\s+",
      "^\\s*%\\s+",
      "^\\s*PASS\\s+",
      "^\\s*\\[[\\d:.]+\\]\\s*$",
    ],
    collapsePatterns: [
      { name: "vitest-pass", pattern: "^\\s*[✓✔]\\s+.*$", replacement: "[vitest] passing test lines collapsed" },
      { name: "vitest-stdio", pattern: "^\\s*(stdout|stderr)\\s*\\|.*$", replacement: "[vitest] stdio chatter collapsed" },
    ],
    maxContextLinesAroundFailure: 3,
  },
  {
    id: "jest",
    label: "Jest",
    autoDetect: {
      commands: ["jest", "npm test", "pnpm jest", "yarn jest"],
      filePatterns: ["jest.config.", ".test.js", ".spec.js", "__tests__/"],
      manifestSignals: ["jest", "ts-jest", "@types/jest"],
      outputPatterns: ["^\\s*FAIL\\s+", "^\\s*expect\\(", "^\\s*Test Suites:"],
    },
    preservePatterns: [
      "^\\s*FAIL\\s+",
      "^\\s*●\\s+",
      "^\\s*(Error|TypeError|ReferenceError|AssertionError)\\b",
      "^\\s*(Expected|Received|expect\\()\\b",
      "^\\s*[-+]\\s",
      "^\\s*>\\s*\\d+\\s*\\|",
      "\\S+\\.(test|spec)\\.(ts|tsx|js|jsx):\\d+:\\d+",
      "^\\s*(Test Suites|Tests|Snapshots|Time):",
      "^\\s*at\\s+",
    ],
    dropPatterns: [
      "^\\s*PASS\\s+",
      "^\\s*console\\.(log|warn|info)",
      "^\\s*Ran all test suites",
      "^\\s*Watch Usage",
      "^\\s*Jest did not exit",
    ],
    collapsePatterns: [
      { name: "jest-pass", pattern: "^\\s*PASS\\s+.*$", replacement: "[jest] passing suites collapsed" },
      { name: "jest-console", pattern: "^\\s*console\\.(log|warn|info).*$", replacement: "[jest] console chatter collapsed" },
    ],
    maxContextLinesAroundFailure: 4,
  },
  {
    id: "pytest",
    label: "Pytest",
    autoDetect: {
      commands: ["pytest", "python -m pytest", "uv run pytest", "poetry run pytest"],
      filePatterns: ["pytest.ini", "pyproject.toml", "test_", "_test.py"],
      manifestSignals: ["pytest", "tool.pytest.ini_options"],
      outputPatterns: ["^={2,}\\s+FAILURES\\s+={2,}", "^FAILED\\s+", "^={2,}\\s+short test summary info\\s+={2,}"],
    },
    preservePatterns: [
      "^={2,}\\s+(FAILURES|ERRORS|short test summary info)\\s+={2,}",
      "^_{2,}\\s+",
      "^FAILED\\s+",
      "^ERROR\\s+",
      "^E\\s+",
      "^>\\s+",
      "\\S+\\.py:\\d+(:|\\s)",
      "^\\s*(AssertionError|ValueError|TypeError|KeyError|RuntimeError)\\b",
      "^\\s*[-+]\\s",
    ],
    dropPatterns: [
      "^\\s*\\.\\.\\.\\s*$",
      "^\\s*[.FE]+\\s+\\[\\s*\\d+%\\]",
      "^={2,}\\s+warnings summary\\s+={2,}",
      "^\\s*-- Docs:",
      "^\\s*collected\\s+\\d+\\s+items",
      "^\\S+\\.py\\s+\\.+\\s+\\[\\s*\\d+%\\]",
    ],
    collapsePatterns: [
      { name: "pytest-progress", pattern: "^\\s*[.FE]+\\s+\\[\\s*\\d+%\\].*$", replacement: "[pytest] progress lines collapsed" },
      { name: "pytest-warnings", pattern: "^={2,}\\s+warnings summary\\s+={2,}.*$", replacement: "[pytest] warning summary collapsed" },
    ],
    maxContextLinesAroundFailure: 5,
  },
  {
    id: "cargo",
    label: "Cargo",
    autoDetect: {
      commands: ["cargo test", "cargo build", "cargo check", "cargo clippy"],
      filePatterns: ["Cargo.toml", "Cargo.lock", "src/lib.rs", "src/main.rs"],
      manifestSignals: ["[package]", "[workspace]", "cargo-features"],
      outputPatterns: ["^error(\\[E\\d+\\])?:", "^thread '.+' panicked", "^failures:"],
    },
    preservePatterns: [
      "^error(\\[E\\d+\\])?:",
      "^warning:",
      "^\\s*-->\\s+\\S+:\\d+:\\d+",
      "^\\s*\\|",
      "^\\s*=\\s+note:",
      "^thread '.+' panicked",
      "^failures:",
      "^----\\s+.+\\s+stdout\\s+----",
      "^test result:\\s+FAILED",
      "^\\s*left:",
      "^\\s*right:",
    ],
    dropPatterns: [
      "^\\s*Compiling\\s+",
      "^\\s*Checking\\s+",
      "^\\s*Finished\\s+",
      "^\\s*Running\\s+",
      "^\\s*Doc-tests\\s+",
      "^test\\s+.+\\s+\\.\\.\\.\\s+ok$",
    ],
    collapsePatterns: [
      { name: "cargo-build-progress", pattern: "^\\s*(Compiling|Checking|Finished|Running|Doc-tests)\\b.*$", replacement: "[cargo] build/test progress collapsed" },
      { name: "cargo-ok-tests", pattern: "^test\\s+.+\\s+\\.\\.\\.\\s+ok$", replacement: "[cargo] passing test lines collapsed" },
    ],
    maxContextLinesAroundFailure: 4,
  },
];

const presetById = new Map(frameworkPresets.map((preset) => [preset.id, preset]));

export function getFrameworkPreset(id) {
  return presetById.get(id);
}

export function normalizePresetMode(value = "auto") {
  if (value === "auto" || value === "none" || FRAMEWORK_IDS.has(value)) return value;
  throw new Error(`Unknown RTK preset mode: ${value}`);
}

export function detectFrameworkPreset({ command = "", files = [], manifests = [], output = "" } = {}) {
  const textParts = [
    command,
    ...files,
    ...manifests.map((entry) => (typeof entry === "string" ? entry : JSON.stringify(entry))),
    output.slice(0, 80_000),
  ];
  const haystack = textParts.join("\n");
  const scores = frameworkPresets.map((preset) => ({
    preset,
    score:
      scoreSignals(command, preset.autoDetect.commands, "command") +
      scoreSignals(files.join("\n"), preset.autoDetect.filePatterns, "file") +
      scoreSignals(manifests.join("\n"), preset.autoDetect.manifestSignals, "manifest") +
      scoreSignals(haystack, preset.autoDetect.outputPatterns, "output"),
  }));
  scores.sort((a, b) => b.score - a.score);
  return scores[0]?.score > 0 ? scores[0].preset.id : undefined;
}

export function chooseFrameworkPreset({ mode = "auto", ...signals } = {}) {
  const normalizedMode = normalizePresetMode(mode);
  if (normalizedMode === "none") {
    return { selectedPreset: "none", reason: "RTK preset filtering disabled" };
  }
  if (FRAMEWORK_IDS.has(normalizedMode)) {
    return {
      selectedPreset: normalizedMode,
      detectedFramework: detectFrameworkPreset(signals),
      reason: `Manual preset selected: ${normalizedMode}`,
    };
  }
  const detectedFramework = detectFrameworkPreset(signals);
  return detectedFramework
    ? { selectedPreset: detectedFramework, detectedFramework, reason: `Detected ${detectedFramework} from command/files/output` }
    : { selectedPreset: "auto", reason: "No framework preset detected" };
}

export function filterFrameworkOutput(output, presetId, options = {}) {
  const preset = getFrameworkPreset(presetId);
  if (!preset) throw new Error(`Unknown RTK preset: ${presetId}`);
  const lines = String(output ?? "").split(/\r?\n/);
  const preserveMatchers = preset.preservePatterns.map((pattern) => re(pattern));
  const dropMatchers = preset.dropPatterns.map((pattern) => re(pattern));
  const context = options.contextLines ?? preset.maxContextLinesAroundFailure;
  const directPreserve = new Set();
  const preserve = new Set();

  lines.forEach((line, index) => {
    if (matchesAny(line, preserveMatchers)) {
      directPreserve.add(index);
      for (let offset = -context; offset <= context; offset += 1) {
        const target = index + offset;
        if (target >= 0 && target < lines.length) preserve.add(target);
      }
    }
  });

  const collapsedCounts = new Map();
  const collapseMatchers = preset.collapsePatterns.map((pattern) => ({
    ...pattern,
    matcher: re(pattern.pattern),
  }));
  const filtered = [];

  lines.forEach((line, index) => {
    if (directPreserve.has(index)) {
      filtered.push(line);
      return;
    }
    const collapse = collapseMatchers.find((pattern) => pattern.matcher.test(line));
    if (collapse) {
      collapsedCounts.set(collapse.replacement, (collapsedCounts.get(collapse.replacement) ?? 0) + 1);
      return;
    }
    if (matchesAny(line, dropMatchers)) return;
    if (preserve.has(index)) {
      filtered.push(line);
      return;
    }
    if (line.trim() === "" && filtered.at(-1)?.trim() === "") return;
    filtered.push(line);
  });

  for (const [replacement, count] of collapsedCounts) {
    filtered.push(`${replacement} (${count})`);
  }

  const filteredOutput = filtered.join("\n").trimEnd();
  return {
    presetId,
    originalBytes: Buffer.byteLength(String(output ?? "")),
    filteredBytes: Buffer.byteLength(filteredOutput),
    estimatedTokensSaved: Math.max(0, Math.round((Buffer.byteLength(String(output ?? "")) - Buffer.byteLength(filteredOutput)) / 4)),
    output: filteredOutput,
    collapsed: Object.fromEntries(collapsedCounts),
  };
}

export function buildRtkPresetDecision({ command = "", mode = "auto", files = [], manifests = [], output = "" } = {}) {
  const selection = chooseFrameworkPreset({ command, mode, files, manifests, output });
  const decision = {
    command,
    selectedPreset: selection.selectedPreset,
    detectedFramework: selection.detectedFramework,
    reason: selection.reason,
  };
  if (selection.selectedPreset !== "auto" && selection.selectedPreset !== "none" && output) {
    const result = filterFrameworkOutput(output, selection.selectedPreset);
    decision.originalBytes = result.originalBytes;
    decision.filteredBytes = result.filteredBytes;
    decision.estimatedTokensSaved = result.estimatedTokensSaved;
  }
  return decision;
}

function scoreSignals(text, patterns, kind) {
  const weight = kind === "command" ? 4 : kind === "manifest" ? 3 : kind === "file" ? 2 : 2;
  return patterns.reduce((score, pattern) => {
    const matcher = pattern.startsWith("^") ? re(pattern, "im") : re(escapeForLoosePattern(pattern), "i");
    return score + (matcher.test(text) ? weight : 0);
  }, 0);
}

function escapeForLoosePattern(pattern) {
  return pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\\\*/g, ".*");
}

function matchesAny(line, matchers) {
  return matchers.some((matcher) => matcher.test(line));
}
