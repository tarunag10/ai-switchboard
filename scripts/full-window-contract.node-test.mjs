import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const config = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
const infoPlist = await readFile("src-tauri/Info.plist", "utf8");
const rustShell = await readFile("src-tauri/src/lib.rs", "utf8");

test("the primary window is a normal resizable Mac application window", () => {
  const main = config.app.windows.find((window) => window.label === "main");
  assert.ok(main, "main window must be configured");
  assert.equal(main.resizable, true);
  assert.equal(main.decorations, true);
  assert.equal(main.skipTaskbar, false);
  assert.ok(main.width >= 1000, "main window must not be constrained to popover width");
  assert.ok(main.minWidth <= main.width);
  assert.ok(main.minHeight <= main.height);
});

test("Dock and menu-bar entry points coexist", () => {
  assert.match(infoPlist, /<key>LSUIElement<\/key>\s*<false\/>/);
  assert.match(rustShell, /ActivationPolicy::Regular/);
  assert.match(rustShell, /RunEvent::Reopen/);
  assert.match(rustShell, /TrayIconBuilder::with_id/);
  assert.match(rustShell, /"run-doctor"/);
  assert.match(rustShell, /"restart-optimizer"/);
});

test("closing hides the window without making loss of focus destructive", () => {
  const eventHandler = rustShell.slice(
    rustShell.indexOf("fn handle_window_event"),
    rustShell.indexOf("struct TraySessionSavings"),
  );
  assert.match(eventHandler, /CloseRequested/);
  assert.match(eventHandler, /prevent_close/);
  assert.match(eventHandler, /window\.hide/);
  assert.doesNotMatch(eventHandler, /Focused\(false\)/);
  assert.doesNotMatch(eventHandler, /stop_headroom/);
});
