// @ts-nocheck
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

describe("repo-intelligence CLI", () => {
  it("emits complete parseable Goose session JSON through stdout pipes", () => {
    const result = spawnSync(
      process.execPath,
      [
        "scripts/repo-intelligence.mjs",
        ".",
        "--session",
        "--agent",
        "goose",
        "--task",
        "verification",
        "--headroom-healthy",
        "--rtk-healthy",
        "--format",
        "json",
      ],
      {
        cwd: repoRoot,
        encoding: "utf8",
        maxBuffer: 2 * 1024 * 1024,
      },
    );

    expect(result.status, result.stderr).toBe(0);
    expect(result.stdout.length).toBeGreaterThan(65_536);

    const payload = JSON.parse(result.stdout);
    expect(payload.copyStatus).toBe("ready");
    expect(payload.repoMapContext.available).toBe(true);
    expect(payload.repoMapContext.compactContextPath).toContain(
      "docs/repo-map/COMPACT_CONTEXT.md",
    );
    expect(payload.repoMapContext.mapPath).toContain("docs/repo-map/repo-map.json");
    expect(payload.repoMapContext.estimatedTokensAvoided).toBeGreaterThan(0);
    expect(payload.handoff.agent.id).toBe("goose");
    expect(payload.handoff.safety.manualProviderRouting).toBe(true);
    expect(payload.configReadiness).toMatchObject({
      plannedConnectorId: "goose",
      automationEnabled: false,
      managedMcpBridge: true,
      supportStatus: "managed_mcp",
    });
    expect(payload.configReadiness.safetyDossier.configPathStrategy).toContain(
      "Repo Memory MCP descriptor",
    );
    expect(payload.configReadiness.safetyDossier.rollbackStrategy).toContain(
      "MCP bridge",
    );
  });

  it("includes Repo Map compact context in markdown packs", () => {
    const result = spawnSync(
      process.execPath,
      [
        "scripts/repo-intelligence.mjs",
        ".",
        "--pack",
        "implementation",
        "--format",
        "markdown",
      ],
      {
        cwd: repoRoot,
        encoding: "utf8",
        maxBuffer: 2 * 1024 * 1024,
      },
    );

    expect(result.status, result.stderr).toBe(0);
    expect(result.stdout).toContain("## Repo Map Compact Context");
    expect(result.stdout).toContain("docs/repo-map/COMPACT_CONTEXT.md");
    expect(result.stdout).toContain("Estimated tokens avoided:");
  });
});
