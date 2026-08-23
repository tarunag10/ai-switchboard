import {
  Bell,
  Brain,
  Calculator,
  Eye,
  FirstAidKit,
  GearSix,
  Graph,
  House,
  PuzzlePiece,
  Sliders,
  type Icon,
} from "@phosphor-icons/react";

import macAiSwitchboardLogo from "../assets/mac-ai-switchboard-logo.png";
import type { TrayView } from "../lib/trayHelpers";

interface NavItem {
  id: TrayView;
  label: string;
  icon: Icon;
}

const navItems: NavItem[] = [
  { id: "home", label: "Overview", icon: House },
  { id: "optimization", label: "Optimize", icon: Sliders },
  { id: "repoIntelligence", label: "Repo Intelligence", icon: Brain },
  { id: "routingModels", label: "Routing", icon: Graph },
  { id: "workbench", label: "Workbench", icon: Graph },
  { id: "usage", label: "Savings", icon: Calculator },
  { id: "doctor", label: "Doctor", icon: FirstAidKit },
  { id: "xray", label: "Token X-Ray", icon: Eye },
  { id: "briefing", label: "Daily Briefing", icon: Graph },
  { id: "agentMemory", label: "Agent Memory", icon: Brain },
  { id: "notifications", label: "Event Log", icon: Bell },
  { id: "repoMap", label: "Repo Map", icon: Graph },
  { id: "addons", label: "Addons", icon: PuzzlePiece },
];

interface TraySidebarProps {
  activeView: TrayView;
  localOnlyMode: boolean;
  onSelectView: (view: TrayView) => void;
}

export function TraySidebar({ activeView, localOnlyMode, onSelectView }: TraySidebarProps) {
  return (
    <aside className="tray-sidebar">
      <div className="tray-sidebar__logo">
        <img src={macAiSwitchboardLogo} alt="AI Switchboard" />
      </div>
      <nav className="tray-nav" aria-label="AI Switchboard navigation">
        {navItems.map((item) => (
          <button
            aria-current={activeView === item.id ? "page" : undefined}
            aria-label={item.label}
            key={item.id}
            className={`tray-nav__item${activeView === item.id ? " is-active" : ""}`}
            onClick={() => onSelectView(item.id)}
            title={item.label}
            type="button"
          >
            <span className="tray-nav__icon" aria-hidden="true">
              <item.icon
                className="tray-nav__icon-svg"
                size={26}
                weight={activeView === item.id ? "fill" : "regular"}
              />
            </span>
            <span className="tray-nav__text">
              <strong>{item.label}</strong>
            </span>
          </button>
        ))}
      </nav>
      <div className="tray-sidebar__footer">
        {!localOnlyMode ? (
          <button
            aria-current={activeView === "upgrade" || activeView === "upgradeAuth" ? "page" : undefined}
            aria-label="Upgrade"
            className={`upgrade-pill${
              activeView === "upgrade" || activeView === "upgradeAuth" ? " is-active" : ""
            }`}
            onClick={() => onSelectView("upgrade")}
            title="Upgrade"
            type="button"
          >
            Upgrade
          </button>
        ) : null}
        <button
          aria-current={activeView === "settings" ? "page" : undefined}
          aria-label="Settings"
          className={`tray-nav__item${activeView === "settings" ? " is-active" : ""}`}
          onClick={() => onSelectView("settings")}
          title="Settings"
          type="button"
        >
          <span className="tray-nav__icon" aria-hidden="true">
            <GearSix
              className="tray-nav__icon-svg"
              size={26}
              weight={activeView === "settings" ? "fill" : "regular"}
            />
          </span>
          <span className="tray-nav__text">
            <strong>Settings</strong>
          </span>
        </button>
      </div>
    </aside>
  );
}
