# Platform Support

AI Switchboard is the parent product. Switchboard is the short name. AI Switchboard for Mac is the packaged desktop app.

| Surface | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Desktop app | Supported first through the DMG build and Tauri app. | Planned. No supported installer yet. | Planned. No supported installer yet. |
| `switchboard` CLI | Runnable provider-neutral Node CLI plus an optional external source-built Rust CLI bridge. No native executable is bundled. | Runnable provider-neutral Node CLI; native Rust CLI is source/CI-supported only and must be supplied externally. | Runnable provider-neutral Node CLI; native Rust CLI is source/CI-supported only and must be supplied externally. |
| Repo Intelligence packs | Supported from app and the Node CLI. | Supported from the Node CLI. | Supported from the Node CLI. |
| Headroom/RTK runtime management | Supported through AI Switchboard for Mac. | Planned. Current docs should treat this as unavailable. | Planned. Current docs should treat this as unavailable. |
| Repair, uninstall, keychain, bundle helpers | macOS-only. Legacy Mac AI Switchboard paths remain compatible. | Not supported. | Not supported. |

Compatibility rules:

- The generated npm tarball is a runnable provider-neutral Node CLI, not a full
  source distribution, desktop app bundle, or native executable package. Its
  shared app manifest still declares frontend dependencies, so it is not an
  offline self-contained npm installation and a normal install may require
  registry access.
- Keep legacy Mac AI Switchboard storage, bundle, keychain, and script paths working.
- Use `switchboard` for CLI examples and keep `npm run repo:intelligence -- ...` as the compatibility path.
- `switchboard harness status` reports the cross-platform local harness and
  Workbench contract separately from native executable availability. The npm
  package does not bundle a native executable; native commands require an
  external absolute `SWITCHBOARD_NATIVE_CLI` path and the exact `--native`
  command form.
- `switchboard harness session <repo-path>` and `switchboard router <repo-path>` prepare the same provider-neutral local session evidence as Repo Intelligence; they do not start a provider process or claim live model routing.
- `switchboard optimize <repo-path>` is the compatibility alias for that local session-planning path. Live Headroom/RTK optimization remains a desktop-runtime capability.
- The standalone Rust CLI in `crates/switchboard-cli` provides three read-only
  stdin/stdout commands: `harness status`, `router endpoint plan`, and
  `workbench session serialize`. It is source-built and has no provider,
  process, filesystem, or Tauri execution surface.
- The package-level Node command delegates to the native Rust CLI only when
  `SWITCHBOARD_NATIVE_CLI` names an explicit executable and the exact
  `--native` form is used:

  ```bash
  SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
    switchboard harness status --native
  SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
    switchboard router endpoint plan --native < endpoint-plan.json
  SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
    switchboard workbench session serialize --native < session.json
  ```

- The bridge does not guess paths or invoke a shell. Legacy
  `router <repo-path> --native` forms and all `optimize --native` forms are
  rejected. Linux and Windows native CLI support is source/CI-supported only;
  no native installer or prebuilt artifact is currently shipped.
- Attribute Headroom, RTK, Ponytail, MarkItDown, and Caveman as integrated upstream tools or add-ons. Do not imply AI Switchboard created them.
