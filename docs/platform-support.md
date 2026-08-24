# Platform Support

AI Switchboard is the parent product. Switchboard is the short name. AI Switchboard for Mac is the packaged desktop app.

| Surface | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Desktop app | Supported first through the DMG build and Tauri app. | Planned. No supported installer yet. | Planned. No supported installer yet. |
| `switchboard` CLI | Repo-local harness/router preview plus Repo Intelligence; keeps `npm run repo:intelligence` compatible. | Same provider-neutral harness/router preview when Node is installed. Desktop/runtime management is not supported yet. | Same provider-neutral harness/router preview when Node is installed. PowerShell/installer support is not supported yet. |
| Repo Intelligence packs | Supported from app and CLI. | Supported from CLI when Node dependencies are installed. | Supported from CLI when Node dependencies are installed. |
| Headroom/RTK runtime management | Supported through AI Switchboard for Mac. | Planned. Current docs should treat this as unavailable. | Planned. Current docs should treat this as unavailable. |
| Repair, uninstall, keychain, bundle helpers | macOS-only. Legacy Mac AI Switchboard paths remain compatible. | Not supported. | Not supported. |

Compatibility rules:

- Keep legacy Mac AI Switchboard storage, bundle, keychain, and script paths working.
- Use `switchboard` for CLI examples and keep `npm run repo:intelligence -- ...` as the compatibility path.
- `switchboard harness status` reports the cross-platform local harness contract.
- `switchboard harness session <repo-path>` and `switchboard router <repo-path>` prepare the same provider-neutral local session evidence as Repo Intelligence; they do not start a provider process or claim live model routing.
- `switchboard optimize <repo-path>` is the compatibility alias for that local session-planning path. Live Headroom/RTK optimization remains a desktop-runtime capability.
- Attribute Headroom, RTK, Ponytail, MarkItDown, and Caveman as integrated upstream tools or add-ons. Do not imply AI Switchboard created them.
