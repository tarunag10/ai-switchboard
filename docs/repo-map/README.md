# Mac AI Switchboard Repo Map

Generated: 2026-07-08T15:33:42.292Z

## Artifacts

- `graphify-out/graph.json`: Graphify AST/knowledge graph output.
- `graphify-out/GRAPH_TREE.html`: Graphify interactive tree view.
- `docs/repo-map/madge-src.json`: TypeScript dependency map.
- `docs/repo-map/dependency-cruiser-src.json`: dependency-cruiser module map.
- `docs/repo-map/cargo-metadata.json`: Rust crate dependency metadata.
- `docs/repo-map/architecture.mmd`: high-level Mermaid architecture.
- `docs/repo-map/repo-map.json`: synthesized machine-readable map.

## Tool Results

- Graphify: partial-success; 4015 nodes, 11057 links.
- Madge: 169 frontend modules, 341 import edges, no cycles found.
- dependency-cruiser: 56 modules, 55 edges.
- Cargo metadata: 40 direct Rust dependencies.

## Shape

- Frontend source files: 167
- Rust source files: 107
- Docs: 35
- Scripts: 62

## Main Runtime Flow

```mermaid
flowchart LR
  User["User"]
  App["src/App.tsx
main React state machine"]
  Components["src/components/*
views and panels"]
  Lib["src/lib/*
copy, helpers, release logic"]
  Assets["src/assets + connectors manifest"]
  Tauri["src-tauri/src/lib.rs
Tauri command handler"]
  RustMods["src-tauri/src/*.rs
proxy, adapters, storage, analytics"]
  OS["macOS / Codex / CLIs / local proxy"]

  User --> App
  App --> Components
  App --> Lib
  Components --> Lib
  Components --> Assets
  App -- invoke(...) --> Tauri
  Components -- invoke(...) --> Tauri
  Tauri --> RustMods
  RustMods --> OS
```

## Frontend Hotspots

- `App.tsx`: imports 55
- `components/HomeView.tsx`: imports 13
- `components/SettingsView.tsx`: imports 11
- `components/AddonsView.tsx`: imports 9
- `components/UsageSavingsView.tsx`: imports 9
- `components/OptimizationView.tsx`: imports 6
- `components/RollbackCenter.tsx`: imports 6
- `components/SwitchboardPanel.tsx`: imports 5
- `lib/doctorRepairCopy.ts`: imports 5
- `lib/headroomLearnController.ts`: imports 5
- `components/DailySavingsChart.tsx`: imports 4
- `components/OptimizationDashboard.tsx`: imports 4
- `components/SavingsCalculatorCard.tsx`: imports 4
- `components/SettingsConnectorPanel.tsx`: imports 4
- `lib/settingsConnectorCopy.test.ts`: imports 4
- `lib/settingsConnectorCopy.ts`: imports 4
- `main.tsx`: imports 4
- `components/ActivityFeed.tsx`: imports 3
- `components/LauncherInstallStep.tsx`: imports 3
- `components/PlannedAddonCard.tsx`: imports 3

## Strongest Folder-Level Edges

- `src/App.tsx -> ./lib`: 31
- `src/App.tsx -> ./components`: 24

## Tauri Command Wiring

- Frontend invokes: 82
- Rust commands declared: 121
- Commands in invoke handler: 122
- Invoked commands missing a Rust command: `start_bootstrap`
- Invoked commands missing from invoke handler: none
- Handler commands not called by current frontend scan: 40

## Direct Dependencies

- NPM runtime: @microsoft/clarity, @phosphor-icons/react, @sentry/react, @tauri-apps/api, @tauri-apps/plugin-dialog, react, react-dom, recharts
- NPM dev: @tauri-apps/cli, @testing-library/jest-dom, @testing-library/react, @testing-library/user-event, @types/react, @types/react-dom, @vitejs/plugin-react, @vitest/coverage-v8, jsdom, postcss, typescript, vite, vitest
- Rust runtime/build/dev direct deps: 40

## Useful Commands

- `npm test -- --run`
- `npm run lint`
- `npm run test:rust`
- `npx --yes madge src --extensions ts,tsx --ts-config tsconfig.json --circular`
- `npx --yes dependency-cruiser src --no-config --output-type json`
- `uvx --from graphifyy graphify . --no-cluster`

