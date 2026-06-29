import { describe, expect, it } from "vitest";

import {
  repoMemoryMcpInspectorRow,
  repoMemoryMcpLifecycle,
} from "./repoMemoryMcp";

describe("repoMemoryMcpLifecycle", () => {
  it("labels configured repo-memory MCP as app-managed and read-only", () => {
    const lifecycle = repoMemoryMcpLifecycle({ configured: true });

    expect(lifecycle).toMatchObject({
      state: "configured",
      status: "Configured",
      installCommand: "install_repo_memory_mcp",
      startCommand: "start_repo_memory_mcp",
      stopCommand: "stop_repo_memory_mcp",
      verifyCommand: "npm run check:repo-memory-mcp",
    });
    expect(lifecycle.detail).toContain("app-managed");
    expect(lifecycle.detail).toContain("read-only");
    expect(lifecycle.copy).toContain("repo_context_pack");
    expect(lifecycle.copy).toContain("repo_symbol_lookup");
    expect(lifecycle.copy).toContain("repo_dependents_of");
  });

  it("labels active repo-memory MCP with start and stop controls", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: true,
      active: true,
      lastStartedAt: "2026-06-28T10:00:00Z",
      lastCheckedAt: "2026-06-28T10:05:00Z",
      supervisionStatus: "verified_active",
    });

    expect(lifecycle).toMatchObject({
      state: "active",
      status: "Active",
      installCommand: "install_repo_memory_mcp",
      startCommand: "start_repo_memory_mcp",
      stopCommand: "stop_repo_memory_mcp",
    });
    expect(lifecycle.detail).toContain("smoke-tested");
    expect(lifecycle.detail).toContain("2026-06-28T10:00:00Z");
    expect(lifecycle.detail).toContain("2026-06-28T10:05:00Z");
    expect(lifecycle.copy).toContain("passed smoke verification");
    expect(lifecycle.copy).toContain("Start action: start_repo_memory_mcp");
    expect(lifecycle.copy).toContain("Stop action: stop_repo_memory_mcp");
  });

  it("requires smoke verification before trusting active MCP state", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: true,
      active: true,
      lastCheckedAt: "2026-06-28T10:04:00Z",
      supervisionStatus: "active",
    });

    expect(lifecycle.state).toBe("unknown");
    expect(lifecycle.status).toBe("Needs verification");
    expect(lifecycle.detail).toContain("smoke verification has not been recorded");
    expect(lifecycle.copy).toContain("run Start MCP again");
  });

  it("requires a fresh Start MCP after app relaunch", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: true,
      active: false,
      lastStartedAt: "2026-06-28T10:00:00Z",
      lastCheckedAt: "2026-06-28T10:08:00Z",
      supervisionStatus: "restart_required",
    });

    expect(lifecycle.state).toBe("restart_required");
    expect(lifecycle.status).toBe("Start required");
    expect(lifecycle.detail).toContain("previous app process");
    expect(lifecycle.detail).toContain("Click Start MCP");
    expect(lifecycle.copy).toContain("fresh app-session start");
    expect(lifecycle.copy).toContain("Start action: start_repo_memory_mcp");
  });

  it("surfaces failed repo-memory MCP smoke checks", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: true,
      active: false,
      lastCheckedAt: "2026-06-28T10:06:00Z",
      supervisionStatus: "smoke_failed",
    });

    expect(lifecycle.state).toBe("smoke_failed");
    expect(lifecycle.status).toBe("Smoke failed");
    expect(lifecycle.detail).toContain("read-only smoke check did not pass");
    expect(lifecycle.detail).toContain("2026-06-28T10:06:00Z");
    expect(lifecycle.copy).toContain("configured state is not enough");
    expect(lifecycle.copy).toContain("repo_context_pack");
  });

  it("surfaces stale active state when MCP config drifts", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: false,
      active: false,
      lastCheckedAt: "2026-06-28T10:07:00Z",
      supervisionStatus: "stale_config",
    });

    expect(lifecycle.state).toBe("stale");
    expect(lifecycle.status).toBe("Stale");
    expect(lifecycle.detail).toContain("marked active");
    expect(lifecycle.detail).toContain("no longer present");
    expect(lifecycle.detail).toContain("2026-06-28T10:07:00Z");
    expect(lifecycle.copy).toContain("active session state no longer matches");
    expect(lifecycle.copy).toContain("Stop action: stop_repo_memory_mcp");
  });

  it("surfaces the one-click prepare action when MCP needs attention", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: false,
      error: "Claude config is missing repo-memory.",
    });

    expect(lifecycle.state).toBe("needs_attention");
    expect(lifecycle.status).toBe("Needs attention");
    expect(lifecycle.detail).toBe("Claude config is missing repo-memory.");
    expect(lifecycle.copy).toContain(
      "Prepare action: install_repo_memory_mcp then start_repo_memory_mcp",
    );
    expect(lifecycle.copy).toContain(
      "Optional terminal verify: npm run check:repo-memory-mcp",
    );
    expect(lifecycle.copy).toContain("secret-like repo paths");
  });

  it("keeps unknown status explicit before lifecycle verification", () => {
    const row = repoMemoryMcpInspectorRow({
      configured: null,
      error: null,
    });

    expect(row).toEqual({
      label: "Repo Memory MCP",
      status: "Unknown",
      detail:
        "Repo Memory MCP lifecycle has not been verified. Use Prepare MCP to install it and run the smoke check before relying on agent MCP handoffs.",
    });
  });
});
