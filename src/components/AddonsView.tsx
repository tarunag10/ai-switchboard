import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import type {
  ClientConnectorStatus,
  DashboardState,
  RuntimeStatus,
} from "../lib/types";
import type { TrayView } from "../lib/trayHelpers";
import {
  buildAddonHealthCards,
  plannedAddons,
  type PlannedAddon,
} from "../lib/plannedAddons";
import { formatPlannedConnectorConfigCreationPlansMarkdown } from "../lib/plannedConnectors";
import { percent1 } from "../lib/dashboardHelpers";
import { AddonHealthStrip } from "./AddonHealthStrip";
import { AddonCard, type AddonCopy } from "./AddonCard";
import { MeasuredAddonSavingsForm } from "./MeasuredAddonSavingsForm";
import { PlannedAddonCard } from "./PlannedAddonCard";

export interface AddonsViewProps {
  activeView: TrayView;
  setActiveView: (view: TrayView) => void;
  addonError: string | null;
  runtimeStatus: RuntimeStatus | null;
  dashboard: DashboardState;
  connectors: ClientConnectorStatus[];
  addonCopy: Record<string, AddonCopy>;
  addonInfoId: string | null;
  setAddonInfoId: React.Dispatch<React.SetStateAction<string | null>>;
  addonBusyId: string | null;
  addonBusyLabel: string | null;
  addonResult: { id: string; message: string } | null;
  setAddonResult: React.Dispatch<
    React.SetStateAction<{ id: string; message: string } | null>
  >;
  rtkAvgSavingsPct: number | null;
  rtkBusy: boolean;
  openExternalLink: (url: string) => Promise<void>;
  runAddonAction: (
    action: "install_addon" | "uninstall_addon" | "set_addon_enabled",
    id: string,
    enabled?: boolean,
  ) => Promise<void>;
  handleRtkToggle: (nextEnabled: boolean) => Promise<void>;
  onMeasuredAddonSavingsRecorded: () => Promise<void>;
  setCavemanLevel: (
    level: "scoped" | "aggressive" | "compact_chinese",
  ) => Promise<void>;
  copyPlannedConnectorCommand: (
    markdown: string,
    label: string,
  ) => Promise<void>;
}

export function AddonsView({
  activeView,
  setActiveView,
  addonError,
  runtimeStatus,
  dashboard,
  connectors,
  addonCopy,
  addonInfoId,
  setAddonInfoId,
  addonBusyId,
  addonBusyLabel,
  addonResult,
  setAddonResult,
  rtkAvgSavingsPct,
  rtkBusy,
  openExternalLink,
  runAddonAction,
  handleRtkToggle,
  onMeasuredAddonSavingsRecorded,
  setCavemanLevel,
  copyPlannedConnectorCommand,
}: AddonsViewProps) {
  const [showRtkDetails, setShowRtkDetails] = useState(false);
  const [rtkActivityLines, setRtkActivityLines] = useState<string[]>([]);
  const rtkActivityRef = useRef<HTMLPreElement | null>(null);

  useEffect(() => {
    if (!showRtkDetails || !rtkActivityRef.current) {
      return;
    }
    rtkActivityRef.current.scrollTop = rtkActivityRef.current.scrollHeight;
  }, [showRtkDetails, rtkActivityLines]);

  return (
    <div className="tray-content" hidden={activeView !== "addons"}>
      <section className="addons">
        <header className="addons__header">
          <h1>Addons</h1>
          <p className="addons__subtitle">
            Installable local add-ons reduce token use and keep
            document/context prep under your control.
          </p>
        </header>
        {addonError ? <p className="addons__error">{addonError}</p> : null}
        <AddonHealthStrip
          cards={buildAddonHealthCards(runtimeStatus, dashboard.tools, {
            dailySavings: dashboard.dailySavings,
            recentUsage: dashboard.recentUsage,
          })}
        />
        <ul className="addons__list">
          <AddonCard
            key="rtk"
            name="RTK"
            version="0.0.0"
            installed={runtimeStatus?.rtk.installed === true}
            enabled={runtimeStatus?.rtk.enabled === true}
            description={
              <>
                Token-optimizing proxy that auto-rewrites your agent's bash
                commands.
                {rtkAvgSavingsPct !== null
                  ? ` ${percent1(rtkAvgSavingsPct)}% avg savings.`
                  : ""}
              </>
            }
            copy={addonCopy.rtk}
            infoOpen={addonInfoId === "rtk"}
            onToggleInfo={() =>
              setAddonInfoId(addonInfoId === "rtk" ? null : "rtk")
            }
            busy={addonBusyId === "rtk"}
            busyLabel={addonBusyLabel}
            resultMessage={
              addonResult?.id === "rtk" ? addonResult.message : null
            }
            onDismissResult={() => setAddonResult(null)}
            sourceUrl={
              dashboard.tools.find((tool) => tool.id === "rtk")
                ?.sourceUrl ?? "https://github.com/rtk-ai/rtk"
            }
            onOpenSource={() =>
              void openExternalLink(
                dashboard.tools.find((tool) => tool.id === "rtk")
                  ?.sourceUrl ?? "https://github.com/rtk-ai/rtk",
              )
            }
            connectors={connectors}
            showClients={
              runtimeStatus?.rtk.installed === true &&
              runtimeStatus.rtk.enabled === true
            }
            actionsDisabled={
              rtkBusy || addonBusyId === "rtk" || !runtimeStatus
            }
            onInstall={() => void runAddonAction("install_addon", "rtk")}
            onToggleEnabled={() =>
              void handleRtkToggle(!runtimeStatus?.rtk.enabled)
            }
            onUninstall={() =>
              void runAddonAction("uninstall_addon", "rtk")
            }
          >
            {runtimeStatus?.rtk.installed ? (
              <>
                <button
                  type="button"
                  className="addon-card__link"
                  onClick={async () => {
                    const next = !showRtkDetails;
                    setShowRtkDetails(next);
                    if (next) {
                      try {
                        const lines = await invoke<string[]>(
                          "get_rtk_activity",
                          { maxLines: 80 },
                        );
                        setRtkActivityLines(lines);
                      } catch {
                        setRtkActivityLines([
                          "Failed to load RTK activity.",
                        ]);
                      }
                    }
                  }}
                >
                  {showRtkDetails
                    ? "Hide RTK activity"
                    : "Show RTK activity"}
                </button>
                {showRtkDetails ? (
                  <pre className="runtime-log" ref={rtkActivityRef}>
                    {rtkActivityLines.length > 0
                      ? rtkActivityLines.join("\n")
                      : "No RTK activity yet."}
                  </pre>
                ) : null}
              </>
            ) : null}
          </AddonCard>
          {dashboard.tools
            .filter((tool) => !tool.required && tool.id !== "rtk")
            .map((tool) => {
              const installed = tool.status !== "not_installed";
              return (
                <AddonCard
                  key={tool.id}
                  name={tool.name}
                  version="0.0.0"
                  installed={installed}
                  enabled={tool.enabled}
                  description={tool.description}
                  copy={addonCopy[tool.id]}
                  infoOpen={addonInfoId === tool.id}
                  onToggleInfo={() =>
                    setAddonInfoId(addonInfoId === tool.id ? null : tool.id)
                  }
                  busy={addonBusyId === tool.id}
                  busyLabel={addonBusyLabel}
                  resultMessage={
                    addonResult?.id === tool.id ? addonResult.message : null
                  }
                  onDismissResult={() => setAddonResult(null)}
                  sourceUrl={tool.sourceUrl}
                  onOpenSource={() => void openExternalLink(tool.sourceUrl)}
                  connectors={connectors}
                  showClients={installed && tool.enabled}
                  actionsDisabled={addonBusyId === tool.id}
                  onInstall={() =>
                    void runAddonAction("install_addon", tool.id)
                  }
                  onToggleEnabled={() =>
                    void runAddonAction(
                      "set_addon_enabled",
                      tool.id,
                      !tool.enabled,
                    )
                  }
                  onUninstall={() =>
                    void runAddonAction("uninstall_addon", tool.id)
                  }
                >
                  {installed &&
                  tool.enabled &&
                  (tool.id === "markitdown" ||
                    tool.id === "ponytail" ||
                    tool.id === "caveman") ? (
                    <MeasuredAddonSavingsForm
                      source={
                        tool.id === "markitdown"
                          ? "markitdown"
                          : tool.id === "ponytail"
                            ? "ponytail"
                            : tool.metadata?.level === "compact_chinese"
                              ? "compact_chinese"
                              : "caveman"
                      }
                      label={tool.name}
                      disabled={addonBusyId === tool.id}
                      onRecorded={onMeasuredAddonSavingsRecorded}
                    />
                  ) : null}
                  {tool.id === "caveman" && installed && tool.enabled
                    ? (() => {
                        const level =
                          tool.metadata?.level === "aggressive"
                            ? "aggressive"
                            : tool.metadata?.level === "compact_chinese"
                              ? "compact_chinese"
                              : "scoped";
                        return (
                          <div className="addon-card__option-block">
                            <div
                              className="addon-card__segmented"
                              role="group"
                              aria-label="Caveman level"
                            >
                              {(
                                [
                                  ["scoped", "Scoped"],
                                  ["aggressive", "Aggressive"],
                                  ["compact_chinese", "Compact Chinese"],
                                ] as const
                              ).map(([item, label]) => (
                                <button
                                  key={item}
                                  type="button"
                                  className={`addon-card__segment${
                                    level === item ? " is-active" : ""
                                  }`}
                                  disabled={
                                    addonBusyId === "caveman" ||
                                    level === item
                                  }
                                  onClick={() => void setCavemanLevel(item)}
                                >
                                  {label}
                                </button>
                              ))}
                            </div>
                            {level === "compact_chinese" ? (
                              <p className="addon-card__hint">
                                Experimental: compact Chinese is only for
                                private internal notes and handoffs. Final
                                user-facing, legal, safety, debugging, and
                                release content stays in the requested
                                language with full detail.
                              </p>
                            ) : null}
                          </div>
                        );
                      })()
                    : null}
                </AddonCard>
              );
            })}
          {plannedAddons.map((addon) => (
            <PlannedAddonCard
              key={addon.id}
              addon={addon}
              onCopyConnectorConfigPlan={(connector) =>
                void copyPlannedConnectorCommand(
                  formatPlannedConnectorConfigCreationPlansMarkdown([
                    connector,
                  ]),
                  `${connector.name} config plan`,
                )
              }
              onOpenRepoIntelligence={() =>
                setActiveView("repoIntelligence")
              }
            />
          ))}
        </ul>
      </section>
    </div>
  );
}
