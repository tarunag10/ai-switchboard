# Platform Support

AI Switchboard is the parent product. Switchboard is the short name. AI Switchboard for Mac is the packaged desktop app.

| Surface | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Desktop app | Supported first through the DMG build and Tauri app. | Planned. No supported installer yet. | Planned. No supported installer yet. |
| `switchboard` CLI | Repo-local preview. Wraps Repo Intelligence and keeps `npm run repo:intelligence` compatible. | Repo-local preview for Node-based Repo Intelligence workflows. Desktop/runtime management is not supported yet. | Repo-local preview where Node scripts run. PowerShell/installer support is not supported yet. |
| Repo Intelligence packs | Supported from app and CLI. | Supported from CLI when Node dependencies are installed. | Supported from CLI when Node dependencies are installed. |
| Headroom/RTK runtime management | Supported through AI Switchboard for Mac. | Planned. Current docs should treat this as unavailable. | Planned. Current docs should treat this as unavailable. |
| Repair, uninstall, keychain, bundle helpers | macOS-only. Legacy Mac AI Switchboard paths remain compatible. | Not supported. | Not supported. |

Compatibility rules:

- Keep legacy Mac AI Switchboard storage, bundle, keychain, and script paths working.
- Use `switchboard` for CLI examples and keep `npm run repo:intelligence -- ...` as the compatibility path.
- Attribute Headroom, RTK, Ponytail, MarkItDown, and Caveman as integrated upstream tools or add-ons. Do not imply AI Switchboard created them.
