# Asset Provenance

Mac AI Switchboard uses app-owned bitmap logo assets in `src/assets/` and `src-tauri/icons/`.

The current logo was generated with ChatGPT image generation, copied into this repository, then resized and converted for launcher, tray, app icon, and packaging use. It is not the inherited Logoipsum/Headroom upstream SVG.

Run `npm run check:branding` before release evidence capture. The guard fails if removed inherited logo names or references reappear.
