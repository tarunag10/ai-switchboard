import assert from "node:assert/strict";
import test from "node:test";
import {
  extractLiteralInvokeCommands,
  extractMountedRouteIds,
  extractRegisteredTauriCommands,
  extractSidebarRouteIds,
  findDeadButtonsInSource,
  findUnmountedSidebarRoutes,
  findUnregisteredInvokes,
  validateConnectorsMount,
  validateRepoMapMount,
} from "./lib/ui-wiring-checks.mjs";

test("Repo Map validation follows the current TrayAppShell mount", () => {
  const shellSource = `
    import { RepoMapView } from "./RepoMapView";
    import { TraySidebar } from "./TraySidebar";
    export function TrayAppShell({ activeView }) {
      return <><TraySidebar /><div hidden={activeView !== "repoMap"}><RepoMapView /></div></>;
    }
  `;
  const sidebarSource = `const navItems = [{ id: "repoMap", label: "Repo Map", icon: Graph }];`;
  assert.deepEqual(validateRepoMapMount({ shellSource, sidebarSource }), []);
});

test("Repo Map validation reports a missing shell render", () => {
  const failures = validateRepoMapMount({
    shellSource: `
      import { RepoMapView } from "./RepoMapView";
      import { TraySidebar } from "./TraySidebar";
      const shell = <><TraySidebar /><div hidden={activeView !== "repoMap"} /></>;
    `,
    sidebarSource: `const navItems = [{ id: "repoMap", label: "Repo Map" }];`,
  });
  assert.deepEqual(failures, ["missing RepoMapView render"]);
});

test("Agents & Connectors validation follows the current TrayApp mount", () => {
  const trayAppSource = `
    import { SettingsConnectorPanel } from "../components/SettingsConnectorPanel";
    const connectorsView = (
      <SettingsConnectorPanel hidden={activeView !== "connectors"} connectors={connectors} />
    );
  `;
  const sidebarSource = `const navItems = [{ id: "connectors", label: "Agents & Connectors", icon: PuzzlePiece }];`;
  assert.deepEqual(validateConnectorsMount({ trayAppSource, sidebarSource }), []);
});

test("Agents & Connectors validation reports a missing mount", () => {
  const failures = validateConnectorsMount({
    trayAppSource: `
      import { SettingsConnectorPanel } from "../components/SettingsConnectorPanel";
      const panel = <SettingsConnectorPanel connectors={connectors} />;
    `,
    sidebarSource: `const navItems = [{ id: "connectors", label: "Agents & Connectors" }];`,
  });
  assert.deepEqual(failures, ["missing Agents & Connectors content pane"]);
});

test("visible sidebar routes must have matching mounted views", () => {
  const sidebarRoutes = extractSidebarRouteIds(`
    const navItems = [
      { id: "home", label: "Home", icon: House },
      { id: "doctor", label: "Doctor", icon: FirstAidKit },
    ];
    <button onClick={() => onSelectView("settings")}>Settings</button>
  `);
  const mountedRoutes = extractMountedRouteIds([
    `<Home hidden={activeView !== "home"} /><Settings hidden={activeView !== "settings"} />`,
  ]);
  assert.deepEqual(findUnmountedSidebarRoutes(sidebarRoutes, mountedRoutes), ["doctor"]);
});

test("literal frontend invokes must exist in the Tauri handler registry", () => {
  const frontendCommands = extractLiteralInvokeCommands(`
    await invoke("get_dashboard_state");
    await invoke<Result>("run_doctor_repair", { action });
    await invoke(\`missing_command\`);
  `);
  const registeredCommands = extractRegisteredTauriCommands(`
    app.invoke_handler(tauri::generate_handler![
      dashboard_commands::get_dashboard_state,
      #[cfg(debug_assertions)]
      switchboard_commands::run_doctor_repair,
    ]);
  `);
  assert.deepEqual(findUnregisteredInvokes(frontendCommands, registeredCommands), ["missing_command"]);
});

test("enabled buttons require a click handler or form-submit contract", () => {
  const source = `
    <button onClick={() => run()}>Run</button>
    <button type="submit">Save</button>
    <button disabled>Status only</button>
    <button className="dead">Dead action</button>
  `;
  assert.deepEqual(findDeadButtonsInSource(source), [5]);
});
