# CLAUDE.md

## Coding Profile

- Prefer the simplest working solution.
- Do not edit blindly; inspect the relevant file first.
- Add comments only when the logic is not obvious.
- For reviews, lead with bugs and concrete fixes.
- For Rust changes, prefer `cargo test --manifest-path src-tauri/Cargo.toml --lib <filter>` or `cargo check --manifest-path src-tauri/Cargo.toml`.
- For frontend changes, run the focused Vitest file or `npm run build` when the surface is shared.

## Repo Notes

- This repo is the Mac AI Switchboard app.
- Use repo-local scripts and docs as truth sources.
- Keep local app state, learned memories, SQLite databases, logs, and generated runtime state out of git.
- Do not commit user-specific absolute paths, local machine names, tokens, credentials, memory databases, or private upstream workflow notes.

## Style Notes

- Use CSS tokens from `src/styles.css` where possible.
- Avoid inline pure `#fff`/`#000` except for launcher/splash one-offs.
- Keep copy factual and avoid overstating measured savings when data is estimated.
