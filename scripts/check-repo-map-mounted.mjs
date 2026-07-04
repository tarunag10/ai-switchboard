#!/usr/bin/env node
import fs from "node:fs";

function fail(message) {
  console.error(`repo map mount check failed: ${message}`);
  process.exit(1);
}

const appSource = fs.readFileSync("src/App.tsx", "utf8");

const requiredSnippets = [
  {
    label: "RepoMapView import",
    snippet: 'RepoMapView } from "./components/RepoMapView";',
  },
  {
    label: "Repo Map nav item",
    snippet: '{ id: "repoMap", label: "Repo Map"',
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

console.log("Repo Map mount OK: sidebar route renders RepoMapView.");
