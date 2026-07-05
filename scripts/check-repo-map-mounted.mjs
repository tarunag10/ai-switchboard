#!/usr/bin/env node
import fs from "node:fs";

function fail(message) {
  console.error(`repo map mount check failed: ${message}`);
  process.exit(1);
}

const appSource = fs.readFileSync("src/App.tsx", "utf8");
const traySidebarSource = fs.readFileSync("src/components/TraySidebar.tsx", "utf8");

const requiredSnippets = [
  {
    label: "RepoMapView import",
    snippet: 'RepoMapView } from "./components/RepoMapView";',
  },
  {
    label: "TraySidebar import",
    snippet: 'TraySidebar } from "./components/TraySidebar";',
  },
  {
    label: "Repo Map content pane",
    snippet: 'hidden={activeView !== "repoMap"}',
  },
  {
    label: "RepoMapView render",
    snippet: "<RepoMapView",
  },
];

for (const { label, snippet } of requiredSnippets) {
  if (!appSource.includes(snippet)) {
    fail(`missing ${label}`);
  }
}

if (!appSource.includes("<TraySidebar")) {
  fail("missing TraySidebar render");
}

if (!traySidebarSource.includes('{ id: "repoMap", label: "Repo Map"')) {
  fail("missing Repo Map nav item");
}

console.log("Repo Map mount OK: sidebar route renders RepoMapView.");
