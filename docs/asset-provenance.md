# Asset Provenance

Mac AI Switchboard uses app-owned bitmap logo assets in `src/assets/` and `src-tauri/icons/`, including the app-owned `src-tauri/icons/mac-ai-switchboard.iconset/` iconset source folder.

The current logo was generated with ChatGPT image generation, copied into this repository, then resized and converted for launcher, tray, app icon, and packaging use. It is not the inherited Logoipsum/Headroom upstream SVG.

Run `npm run check:branding` before release evidence capture. The guard fails if removed inherited logo names, the old inherited iconset folder name, or removed upstream references reappear.
