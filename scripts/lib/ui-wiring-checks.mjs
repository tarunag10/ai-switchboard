import fs from "node:fs";
import path from "node:path";

const FRONTEND_EXTENSIONS = new Set([".ts", ".tsx"]);
const TEST_FILE_PATTERN = /(?:^|\/)[^/]+\.(?:test|spec)\.[cm]?[jt]sx?$/;

export function extractLiteralInvokeCommands(source) {
  const commands = new Set();
  const invokePattern = /\binvoke(?:\s*<[^;()\n]+>)?\s*\(\s*(["'`])([A-Za-z0-9_]+)\1/g;

  for (const match of source.matchAll(invokePattern)) {
    commands.add(match[2]);
  }

  return commands;
}

export function extractRegisteredTauriCommands(source) {
  const marker = "tauri::generate_handler![";
  const start = source.indexOf(marker);
  if (start === -1) {
    throw new Error("missing tauri::generate_handler! registry");
  }

  const bodyStart = start + marker.length;
  let depth = 1;
  let end = bodyStart;
  for (; end < source.length && depth > 0; end += 1) {
    if (source[end] === "[") depth += 1;
    if (source[end] === "]") depth -= 1;
  }
  if (depth !== 0) {
    throw new Error("unterminated tauri::generate_handler! registry");
  }

  const body = source
    .slice(bodyStart, end - 1)
    .replace(/#\[[^\]]+\]/g, "");
  const commands = new Set();
  const entryPattern = /(?:^|,)\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Za-z_][A-Za-z0-9_]*)\s*(?=,|$)/gm;

  for (const match of body.matchAll(entryPattern)) {
    commands.add(match[1]);
  }

  return commands;
}

export function findUnregisteredInvokes(frontendCommands, registeredCommands) {
  return [...frontendCommands]
    .filter((command) => !registeredCommands.has(command))
    .sort();
}

export function extractSidebarRouteIds(source) {
  const routes = new Set();
  const navItemPattern = /\{\s*id:\s*["']([A-Za-z0-9_]+)["']\s*,\s*label:/g;
  const directSelectPattern = /onSelectView\(\s*["']([A-Za-z0-9_]+)["']\s*\)/g;

  for (const match of source.matchAll(navItemPattern)) routes.add(match[1]);
  for (const match of source.matchAll(directSelectPattern)) routes.add(match[1]);
  return routes;
}

export function extractMountedRouteIds(sources) {
  const routes = new Set();
  const mountPattern = /hidden\s*=\s*\{\s*activeView\s*!==\s*["']([A-Za-z0-9_]+)["']\s*\}/g;

  for (const source of sources) {
    for (const match of source.matchAll(mountPattern)) routes.add(match[1]);
  }
  return routes;
}

export function findUnmountedSidebarRoutes(sidebarRoutes, mountedRoutes) {
  return [...sidebarRoutes]
    .filter((route) => !mountedRoutes.has(route))
    .sort();
}

export function validateRepoMapMount({ shellSource, sidebarSource }) {
  const failures = [];
  if (!/import\s*\{[^}]*\bRepoMapView\b[^}]*\}\s*from\s*["']\.\/RepoMapView["']/.test(shellSource)) {
    failures.push("missing RepoMapView import in TrayAppShell");
  }
  if (!/import\s*\{[^}]*\bTraySidebar\b[^}]*\}\s*from\s*["']\.\/TraySidebar["']/.test(shellSource)) {
    failures.push("missing TraySidebar import in TrayAppShell");
  }
  if (!/hidden\s*=\s*\{\s*activeView\s*!==\s*["']repoMap["']\s*\}/.test(shellSource)) {
    failures.push("missing Repo Map content pane");
  }
  if (!/<RepoMapView\b/.test(shellSource)) {
    failures.push("missing RepoMapView render");
  }
  if (!/<TraySidebar\b/.test(shellSource)) {
    failures.push("missing TraySidebar render");
  }
  if (!extractSidebarRouteIds(sidebarSource).has("repoMap")) {
    failures.push("missing Repo Map nav item");
  }
  return failures;
}

export function listFrontendSourceFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolutePath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
      } else if (
        FRONTEND_EXTENSIONS.has(path.extname(entry.name)) &&
        !TEST_FILE_PATTERN.test(absolutePath)
      ) {
        files.push(absolutePath);
      }
    }
  };
  visit(path.join(root, "src"));
  return files.sort();
}

export function collectLiteralFrontendInvokes(root) {
  const commands = new Set();
  for (const file of listFrontendSourceFiles(root)) {
    const source = fs.readFileSync(file, "utf8");
    for (const command of extractLiteralInvokeCommands(source)) commands.add(command);
  }
  return commands;
}
