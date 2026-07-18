/** A deliberately small, dependency-free seam for a future chonkify adapter. */

export type EvidenceLabel = "estimated";

export interface RepoSourceSpan {
  repositoryRelativePath: string;
  startLine: number;
  endLine: number;
}

export interface RepoPackCompressionMetadata {
  sourceSpans: RepoSourceSpan[];
  sourceContentHash: string;
  compressor: string;
  compressorVersion: string;
  configHash: string;
  outputHash: string;
  skippedFiles: Array<{ repositoryRelativePath: string; reason: string }>;
  safety: { noNetwork: true; noModel: true };
  evidence: { label: EvidenceLabel };
  license: {
    status: "blocked" | "verified";
    reason: string;
  };
}

export interface RepoPackCompressionResult<TPack> {
  pack: TPack;
  enabled: boolean;
  blocked: boolean;
  blockedReason: string | null;
  metadata: RepoPackCompressionMetadata | null;
}

export interface RepoPackChonkifyAdapter<TPack> {
  readonly name: string;
  readonly version: string;
  compress(input: {
    currentPack: TPack;
    files: readonly RepoPackSourceFile[];
    config: unknown;
  }): {
    pack: TPack;
    sourceSpans: readonly RepoSourceSpan[];
    skippedFiles: readonly { repositoryRelativePath: string; reason: string }[];
  };
}

export interface RepoPackSourceFile {
  repositoryRelativePath: string;
  content: string;
  startLine?: number;
  endLine?: number;
}

export interface RepoPackCompressionOptions<TPack> {
  currentPack: TPack;
  files: readonly RepoPackSourceFile[];
  config?: unknown;
  enabled?: boolean;
  licenseMetadata?: string;
  adapter?: RepoPackChonkifyAdapter<TPack>;
}

export const CHONKIFY_LICENSE_BLOCKED_REASON = "Current license metadata is NOASSERTION; compression is blocked." as const;
const LICENSE_EVIDENCE_REASON = "Chonkify license and provenance evidence is required before compression can be enabled." as const;

/** Stable, synchronous hash for provenance and fixture portability (not cryptographic). */
export function deterministicHash(value: string): string {
  const hashes = [2166136261, 2246822519, 3266489917, 668265263];
  for (let i = 0; i < value.length; i += 1) {
    const code = value.charCodeAt(i);
    for (let h = 0; h < hashes.length; h += 1) {
      hashes[h] ^= code + h * 17;
      hashes[h] = Math.imul(hashes[h], 16777619 + h * 2) >>> 0;
    }
  }
  return hashes.map((hash) => hash.toString(16).padStart(8, "0")).join("");
}

function canonical(value: unknown): string {
  if (value === undefined) return "undefined";
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  return `{${Object.keys(value as object).sort().map((key) => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`).join(",")}}`;
}

export function compressRepoPack<TPack>(options: RepoPackCompressionOptions<TPack>): RepoPackCompressionResult<TPack> {
  const disabled = !options.enabled || !options.adapter;
  if (disabled) return { pack: options.currentPack, enabled: false, blocked: false, blockedReason: null, metadata: null };

  if (options.licenseMetadata !== "MIT") {
    const blockedReason = options.licenseMetadata === "NOASSERTION"
      ? CHONKIFY_LICENSE_BLOCKED_REASON
      : LICENSE_EVIDENCE_REASON;
    return { pack: options.currentPack, enabled: true, blocked: true, blockedReason, metadata: null };
  }

  if (!options.adapter) {
    return { pack: options.currentPack, enabled: false, blocked: false, blockedReason: null, metadata: null };
  }
  const adapter = options.adapter;
  const files = [...options.files].sort((a, b) => a.repositoryRelativePath.localeCompare(b.repositoryRelativePath));
  const sourceContentHash = deterministicHash(files.map((file) => `${file.repositoryRelativePath}\0${file.content}`).join("\n"));
  const configHash = deterministicHash(canonical(options.config ?? {}));
  const compressed = adapter.compress({ currentPack: options.currentPack, files, config: options.config ?? {} });
  const outputHash = deterministicHash(canonical(compressed.pack));
  return {
    pack: compressed.pack,
    enabled: true,
    blocked: false,
    blockedReason: null,
    metadata: {
      sourceSpans: [...compressed.sourceSpans], sourceContentHash, compressor: adapter.name,
      compressorVersion: adapter.version, configHash, outputHash,
      skippedFiles: [...compressed.skippedFiles], safety: { noNetwork: true, noModel: true },
      evidence: { label: "estimated" }, license: { status: "verified", reason: "License metadata supplied as MIT; release provenance still requires review." },
    },
  };
}

export function describeRepoPackCompressionState(): string {
  return "Pack compression: off; deterministic native Repo Intelligence output is preserved. Chonkify remains blocked until license and provenance evidence pass.";
}
