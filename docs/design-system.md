# AI Switchboard interface doctrine

This document is the visual and interaction contract for every AI Switchboard surface. The product uses one **warm control-room** language: dark walnut working surfaces, brass primary actions, teal verified states, coral failures, and blue keyboard focus. New UI must extend this system rather than introduce a screen-local theme.

## Source of truth

- `src/styles/tokens.css` retains compatibility tokens used by older components.
- `src/switchboard-theme.css` contains the product's decorative control-room treatment.
- `src/styles/design-system.css` is imported last and is the canonical cross-screen contract. It supplies missing semantic tokens and normalizes controls, surfaces, contrast, focus, disabled states, and responsive behavior.
- Component stylesheets own local layout only. Production components must not inject runtime `<style>` elements or use hard-coded light surfaces.

## Color roles

| Role | Contract |
| --- | --- |
| Application and navigation | Near-black walnut; navigation remains visually separate from the work surface. |
| Cards and panels | Dark brown elevated surfaces with brass-tinted borders. Never white or cream. |
| Primary action | Brass-to-copper gradient with dark text. Yellow/brass buttons never use white text. |
| Secondary action | Dark copper surface with warm off-white text. |
| Success / verified | Teal. Do not use teal for destructive or pending states. |
| Warning | Bright brass. Warning copy must retain readable dark or warm-light contrast for its surface. |
| Failure / destructive | Coral. A failure also needs text or an icon; color alone is insufficient. |
| Keyboard focus | Blue outer ring with at least 2 px visible thickness. |

## Typography and spacing

- Use the shared `--text-*` scale. Navigation labels must never be smaller than `--text-xs`.
- Body copy uses `--text-sm` or larger and a minimum 1.35 line height.
- Controls have a minimum 38 px height in primary workflows and preserve at least 10 px horizontal breathing room.
- Navigation must show a readable label without clipping at the normal desktop window width. Compact layouts may shorten labels but must retain an accessible name.
- Cards use an 8–10 px radius and deliberate 10–16 px internal spacing. Avoid isolated oversized white blocks or decorative glass panels.

## Component behavior

Every button must provide:

1. A visible default, hover, pressed, focus, busy, disabled, success, and failure state where applicable.
2. An accessible name that describes the action rather than the icon.
3. Immediate feedback after activation. Long-running actions change their label and expose `aria-busy`; failures appear near the initiating control.
4. A real handler or an explicit disabled reason. Placeholder controls are not permitted in production views.
5. Black/dark text when the background is yellow or brass.

## Surface and contrast rules

- The product theme is explicit and does not depend on the Mac's light/dark appearance.
- Text and interactive controls should meet WCAG AA contrast (4.5:1 for ordinary text, 3:1 for large text and graphical controls).
- `--surface-elevated`, `--surface-muted`, `--surface-overlay`, `--text-primary`, and `--text-secondary` are the only normal card-depth roles. New aliases require a documented role.
- Empty, loading, blocked, and unavailable states remain visible and honest; never hide text by matching its panel background.

## Review checklist

- Inspect every sidebar route at the default window size and at 760 px and 520 px widths.
- Keyboard-tab through every visible action and verify the focus ring remains inside the viewport.
- Confirm every frontend native `invoke()` appears in the Tauri handler registry.
- Exercise action success, failure, busy, disabled, and retry paths in component or integration tests.
- Run `npm run check:ui-wiring`, the frontend suite with coverage, and `npm run build` before release.
- Validate the installed native build separately; source-level browser tests do not prove macOS permissions, Keychain, signing, or runtime services.
