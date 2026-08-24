# Platform Support

AI Switchboard is the parent product. Switchboard is the short name. AI Switchboard for Mac is the packaged desktop app.

| Surface | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Desktop app | Supported first through the DMG build and Tauri app. | Planned. No supported installer yet. | Planned. No supported installer yet. |
| `switchboard` CLI | Node Repo Intelligence preview plus a source-built native Rust CLI contract. The package command keeps `npm run repo:intelligence` compatible. | Node preview works when dependencies are installed; native Rust CLI is source/CI-supported only. No native installer or artifact is shipped. | Node preview works when dependencies are installed; native Rust CLI is source/CI-supported only. No native installer or artifact is shipped. |
| Repo Intelligence packs | Supported from app and CLI. | Supported from CLI when Node dependencies are installed. | Supported from CLI when Node dependencies are installed. |
| Headroom/RTK runtime management | Supported through AI Switchboard for Mac. | Planned. Current docs should treat this as unavailable. | Planned. Current docs should treat this as unavailable. |
| Repair, uninstall, keychain, bundle helpers | macOS-only. Legacy Mac AI Switchboard paths remain compatible. | Not supported. | Not supported. |

Compatibility rules:

- Keep legacy Mac AI Switchboard storage, bundle, keychain, and script paths working.
- Use `switchboard` for CLI examples and keep `npm run repo:intelligence -- ...` as the compatibility path.
- `switchboard harness status` reports the cross-platform local harness contract.
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
