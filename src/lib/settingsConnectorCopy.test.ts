import { describe, expect, it } from "vitest";
import {
  firstManagedConfigTarget,
  formatBackendConnectorConfigPlan,
  getConnectorDetectionWarning,
  getConnectorUnavailableReason,
  getPlannedConnectorNextStep,
  supportsNativeConfigApply,
  supportsNativeManagedRollback,
} from "./settingsConnectorCopy";
import { managedChangeRecords } from "./managedChanges";
import {
  managedConnectorDossiers,
  plannedConnectors,
} from "./plannedConnectors";
import type { ClientConnectorStatus } from "./types";

const connector = (
  overrides: Partial<ClientConnectorStatus>,
): ClientConnectorStatus => ({
  clientId: "cursor",
  name: "Cursor",
  installed: true,
  enabled: false,
  verified: false,
  ...overrides,
});

describe("settings connector copy", () => {
  it("summarizes unavailable and disabled connector states", () => {
    const missingCursor = connector({ installed: false });

    expect(getConnectorDetectionWarning(missingCursor)).toContain(
      "Cursor automatic setup is off",
    );
    expect(getConnectorUnavailableReason(missingCursor)).toContain(
      "not detected",
    );
  });

  it("uses shared planned connector next-step copy", () => {
    const opencode = managedConnectorDossiers.find(
      (dossier) => dossier.id === "opencode",
    );
    const cursor = plannedConnectors.find((dossier) => dossier.id === "cursor");

    expect(opencode).toBeDefined();
    expect(cursor).toBeDefined();
    expect(
      getPlannedConnectorNextStep(
        connector({
          clientId: "opencode",
          name: "OpenCode",
          installed: true,
        }),
        opencode!,
      ),
    ).toContain("Managed routing can be repaired");
    expect(
      getPlannedConnectorNextStep(
        connector({ clientId: "cursor", name: "Cursor", installed: true }),
        cursor!,
      ),
    ).toContain("App-guided setup is next");
  });

  it("formats backend config plans with dry-run evidence", () => {
    const cursor = plannedConnectors.find((dossier) => dossier.id === "cursor");

    expect(cursor).toBeDefined();
    expect(
      formatBackendConnectorConfigPlan(
        connector({
          clientId: "cursor",
          name: "Cursor",
          configCreationStepDetails: [
            {
              id: "detect",
              label: "Detect config",
              detail: "Find the active profile.",
              requiredEvidence: ["settings.json"],
            },
          ],
          configDryRunPreview: {
            target: "~/Library/Application Support/Cursor",
            marker: "mac-ai-switchboard:cursor",
            backupPath: "~/backup",
            currentState: "manual",
            proposedState: "managed",
            applyBlockedReason: "Not promoted",
            rollbackPreview: "restore backup",
            confirmationPhrase: "APPLY CURSOR CONFIG",
            writes: ["settings.json"],
          },
        }),
        cursor!,
      ),
    ).toContain("Required evidence: settings.json");
  });

  it("keeps native apply and rollback support decisions centralized", () => {
    const opencode = managedChangeRecords.find(
      (record) => record.id === "opencode-routing",
    );
    const storage = managedChangeRecords.find(
      (record) => record.id === "managed-storage",
    );
    const goose = managedChangeRecords.find(
      (record) => record.id === "goose-provider-routing",
    );

    expect(opencode).toBeDefined();
    expect(storage).toBeDefined();
    expect(goose).toBeDefined();
    expect(firstManagedConfigTarget(opencode!)).toContain("opencode");
    expect(supportsNativeConfigApply(opencode!)).toBe(true);
    expect(supportsNativeConfigApply(goose!)).toBe(true);
    expect(supportsNativeConfigApply(storage!)).toBe(false);
    expect(supportsNativeManagedRollback(opencode!)).toBe(true);
    expect(supportsNativeManagedRollback(storage!)).toBe(true);
  });
});
