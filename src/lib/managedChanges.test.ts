import { describe, expect, it } from "vitest";
import {
  buildManagedConfigDiffPreview,
  formatManagedConfigDiffPreview,
  managedChangeRecords,
} from "./managedChanges";

describe("managedChangeRecords", () => {
  it("keeps rollback-center inventory stable and app-owned", () => {
    expect(managedChangeRecords.map((record) => record.id)).toEqual([
      "client-hooks",
      "managed-hooks",
      "managed-storage",
      "repo-intelligence",
      "login-item",
      "app-state",
      "plugins-backups",
    ]);
    expect(
      managedChangeRecords.every((record) => record.rollback.length > 0),
    ).toBe(true);
  });

  it("tracks marker, backup, and verification evidence for every managed change", () => {
    expect(managedChangeRecords.every((record) => record.markerId.length > 0)).toBe(
      true,
    );
    expect(
      managedChangeRecords.every((record) => record.lastVerifiedLabel.length > 0),
    ).toBe(true);
    expect(
      managedChangeRecords
        .filter((record) => record.kind === "client_config" || record.kind === "shell_hook")
        .every((record) => record.backupPath?.includes("*.headroom.bak")),
    ).toBe(true);
  });

  it("covers reversible config storage footprint", () => {
    const allCopy = managedChangeRecords
      .flatMap((record) => [
        record.owner,
        record.text,
        record.rollback,
        record.markerId,
        record.backupPath ?? "",
        record.lastVerifiedLabel,
        ...record.paths,
      ])
      .join(" ");

    expect(allCopy).toContain("Claude Code");
    expect(allCopy).toContain("Codex");
    expect(allCopy).toContain("~/Library/Application Support/Headroom");
    expect(allCopy).toContain("Repo Intelligence");
    expect(allCopy).toContain("User repositories are not modified");
    expect(allCopy).toContain("~/Library/LaunchAgents/");
    expect(allCopy).toContain("Keychain");
    expect(allCopy).toContain("Ponytail");
    expect(allCopy).toContain("headroom:client-routing");
    expect(allCopy).toContain("*.headroom.bak");
  });

  it("builds a safe dry-run diff preview for managed config edits", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "client-hooks",
    );
    expect(record).toBeDefined();

    const preview = buildManagedConfigDiffPreview({
      record: record!,
      targetPath: " ~/.codex/config.toml ",
      currentManagedBlock: " # >>> headroom:client-routing >>>\nold\n# <<< headroom:client-routing <<< ",
      proposedManagedBlock:
        "# >>> headroom:client-routing >>>\nnew\n# <<< headroom:client-routing <<<",
    });

    expect(preview).toMatchObject({
      recordId: "client-hooks",
      owner: "Headroom engine routing",
      targetPath: "~/.codex/config.toml",
      markerId: "headroom:client-routing",
      backupPath: "next to edited client config as *.headroom.bak",
    });
    expect(preview.currentManagedBlock).toContain("old");
    expect(preview.proposedManagedBlock).toContain("new");
    expect(preview.safetyNotes.join(" ")).toContain("dry-run diff");
    expect(preview.safetyNotes.join(" ")).toContain("Off mode");
  });

  it("formats managed config diff previews for review before writes", () => {
    const record = managedChangeRecords.find(
      (candidate) => candidate.id === "managed-hooks",
    )!;
    const text = formatManagedConfigDiffPreview(
      buildManagedConfigDiffPreview({
        record,
        targetPath: "~/.zshrc",
        currentManagedBlock: null,
        proposedManagedBlock:
          "# >>> headroom:rtk >>>\nsource ~/.headroom/rtk.sh\n# <<< headroom:rtk <<<",
      }),
    );

    expect(text).toContain("Managed config diff: RTK shell compression");
    expect(text).toContain("Target: ~/.zshrc");
    expect(text).toContain("Current managed block:\n(none detected)");
    expect(text).toContain("Proposed managed block:");
    expect(text).toContain("Backup: next to edited shell profile as *.headroom.bak");
    expect(text).toContain("Off mode must remove only Switchboard-owned");
  });

  it("rejects diff previews for non-config records or missing inputs", () => {
    const storage = managedChangeRecords.find(
      (candidate) => candidate.id === "managed-storage",
    )!;
    expect(() =>
      buildManagedConfigDiffPreview({
        record: storage,
        targetPath: "~/Library/Application Support/Headroom",
        proposedManagedBlock: "managed",
      }),
    ).toThrow("does not require a config backup");

    const clientHooks = managedChangeRecords.find(
      (candidate) => candidate.id === "client-hooks",
    )!;
    expect(() =>
      buildManagedConfigDiffPreview({
        record: clientHooks,
        targetPath: "",
        proposedManagedBlock: "managed",
      }),
    ).toThrow("targetPath is required");
    expect(() =>
      buildManagedConfigDiffPreview({
        record: clientHooks,
        targetPath: "~/.codex/config.toml",
        proposedManagedBlock: " ",
      }),
    ).toThrow("proposedManagedBlock is required");
  });
});
