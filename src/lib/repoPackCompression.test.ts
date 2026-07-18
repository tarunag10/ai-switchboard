import { describe, expect, it, vi } from "vitest";
import { compressRepoPack, deterministicHash, type RepoPackChonkifyAdapter } from "./repoPackCompression";

const adapter: RepoPackChonkifyAdapter<{ text: string }> = {
  name: "test-chonkify", version: "0.1.0",
  compress: ({ files }) => ({
    pack: { text: files.map((file) => file.content).join("\n") },
    sourceSpans: files.map((file) => ({ repositoryRelativePath: file.repositoryRelativePath, startLine: 1, endLine: file.content.split("\n").length })),
    skippedFiles: [{ repositoryRelativePath: "dist/app.js", reason: "generated file" }],
  }),
};

describe("repoPackCompression", () => {
  it("preserves the exact pack and does not call the adapter when off", () => {
    const currentPack = { text: "current", nested: { stable: true } };
    const compress = vi.fn(adapter.compress);
    const result = compressRepoPack({ currentPack, files: [], enabled: false, adapter: { ...adapter, compress } });
    expect(result.pack).toBe(currentPack);
    expect(result.metadata).toBeNull();
    expect(compress).not.toHaveBeenCalled();
  });

  it("produces stable provenance and output hashes", () => {
    const input = { currentPack: { text: "current" }, enabled: true, adapter, licenseMetadata: "MIT", config: { width: 80 }, files: [{ repositoryRelativePath: "src/a.ts", content: "a\nb" }] };
    const first = compressRepoPack(input);
    const second = compressRepoPack({ ...input, files: [...input.files].reverse() });
    expect(first).toEqual(second);
    expect(first.metadata).toMatchObject({ compressor: "test-chonkify", compressorVersion: "0.1.0", evidence: { label: "estimated" }, safety: { noNetwork: true, noModel: true }, skippedFiles: [{ reason: "generated file" }] });
    expect(first.metadata?.sourceSpans[0]).toEqual({ repositoryRelativePath: "src/a.ts", startLine: 1, endLine: 2 });
    expect(first.metadata?.outputHash).toBe(second.metadata?.outputHash);
  });

  it("blocks NOASSERTION metadata and keeps the current pack", () => {
    const currentPack = { text: "current" };
    const result = compressRepoPack({ currentPack, files: [], enabled: true, adapter, licenseMetadata: "NOASSERTION" });
    expect(result.pack).toBe(currentPack);
    expect(result.blocked).toBe(true);
    expect(result.blockedReason).toBe("Current license metadata is NOASSERTION; compression is blocked.");
    expect(result.metadata).toBeNull();
  });
});
