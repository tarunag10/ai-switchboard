import { describe, expect, it } from "vitest";
import { managedChangeRecords } from "./managedChanges";

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
});
