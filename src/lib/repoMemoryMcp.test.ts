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
      verifyCommand: "npm run check:repo-memory-mcp",
    });
    expect(lifecycle.detail).toContain("app-managed");
    expect(lifecycle.detail).toContain("read-only");
    expect(lifecycle.copy).toContain("repo_context_pack");
    expect(lifecycle.copy).toContain("repo_symbol_lookup");
    expect(lifecycle.copy).toContain("repo_dependents_of");
  });

  it("surfaces install and smoke commands when MCP needs attention", () => {
    const lifecycle = repoMemoryMcpLifecycle({
      configured: false,
      error: "Claude config is missing repo-memory.",
    });

    expect(lifecycle.state).toBe("needs_attention");
    expect(lifecycle.status).toBe("Needs attention");
    expect(lifecycle.detail).toBe("Claude config is missing repo-memory.");
    expect(lifecycle.copy).toContain("Install action: install_repo_memory_mcp");
    expect(lifecycle.copy).toContain("Verify: npm run check:repo-memory-mcp");
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
        "Repo Memory MCP lifecycle has not been verified. Run the installer and smoke check before relying on agent MCP handoffs.",
    });
  });
});
