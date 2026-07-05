/// Receipts strictly below this version cannot be safely upgraded in place to
/// the currently bundled headroom-ai — pip's in-place upgrade leaves stale
/// `.so`/`.dylib` files from old native-extension pins (onnxruntime,
/// tokenizers, cryptography, mmh3, py_rust_stemmers, uvloop/httptools)
/// alongside the new ones, which surfaces as "smoke test passes, boot
/// validation fails with no log lines and no port bound" — the python
/// process segfaults on import before reaching logging setup.
///
/// Bumping this floor is a release-by-release decision: when a new lock
/// adds native deps or bumps native pins ABI-incompatibly, raise the floor
/// to the previous bundled version. When the new lock only churns pure-Python
/// pins, leave the floor where it is.
///
/// Floor history:
/// - 0.3.7: set to 0.10.0. 0.3.6's lock jump from 0.8.2 → 0.19.0 added
///   fastembed/mmh3/py_rust_stemmers and bumped tokenizers/cryptography/
///   uvicorn; the failing Sentry users all had `fallback: 0.8.2` (from
///   0.2.50-era desktop). 0.3.0-rc.26 onward shipped headroom-ai 0.10.x
///   against the same lock as 0.8.2 — these users have the same dep set
///   on disk and have not produced upgrade-failure events, so we let them
///   take the cheap in-place path. If 0.10.x fallbacks start appearing in
///   Sentry, raise the floor.
/// - 0.3.8: a single `fallback: 0.10.12` boot-validation stall appeared in
///   Sentry, but a clean-VM 0.3.5 → 0.3.7 upgrade reproduced the same
///   0.10.12 → 0.19.0 in-place delta and succeeded. The N=1 failure looks
///   environmental, not universal to the 0.10.x cohort. With the new
///   "Retry with full rebuild" button as a recovery path for the
///   environmental cases, we keep the floor at 0.10.0 rather than penalize
///   the (probably ~99%) of 0.10.x users who succeed in-place. Re-evaluate
///   if multi-machine 0.10.x failures show up in 0.3.8 telemetry.
/// - 0.4.0: raised to 0.20.0. Upstream 0.20.x switched headroom-ai to a
///   maturin/Rust-native single-wheel build (upstream #355) — wheels are now
///   per-Python-version and per-platform and ship a compiled `headroom_core`
///   `.so`. 0.19.0 venvs were built against a `py3-none-any` wheel with no
///   native extension; an in-place pip upgrade onto the new wheel would
///   layer the new `.so` on top of stale transitive native pins from the
///   old lock, which is the exact segfault-on-import pattern this floor
///   exists to prevent. Atomic rebuild is the only safe path for the
///   0.10.x–0.19.x cohort on this bump.
/// - 0.4.2 (0.24.0 → 0.25.0 bundle): floor stays at 0.20.0. The lock delta is
///   two pure-Python pins (litellm, importlib-metadata); no native pin moves
///   and no native dep is added or removed. The headroom-ai wheel itself goes
///   from a per-version `cp312-cp312` native wheel to a stable-ABI `cp310-abi3`
///   wheel (upstream #516), but pip uninstalls the old wheel (clearing its
///   `cpython-312` `.so` via RECORD) before unpacking the new `abi3` `.so`, so
///   no stale headroom_core extension is layered. The 0.20.x+ cohort — which
///   includes every 0.24.0-shipping (desktop 0.4.1) user — upgrades in place
///   with no big-wheel rebuilds. Raise the floor only if a future lock adds or
///   ABI-bumps a native transitive dep.
/// - 0.4.x (0.25.0 → 0.26.0 bundle): floor stays at 0.20.0. headroom-ai's
///   `requires_dist` is byte-identical between 0.25.0 and 0.26.0, so the lock
///   is reused unchanged — no pin moves at all. pip uninstalls the 0.25.0 abi3
///   wheel (clearing its `.so` via RECORD) and unpacks the 0.26.0 abi3 wheel;
///   no stale native extension is layered. The 0.20.x+ cohort upgrades in
///   place with no wheel rebuilds.
/// - 0.4.x (0.26.0 → 0.27.0 bundle): floor stays at 0.20.0. The lock moves three
///   pins: tree-sitter-language-pack 1.8.1 → 0.13.0 (native, ships compiled
///   grammar `.so`s) and the new spreadsheet extra (et-xmlfile/openpyxl/xlrd,
///   pure-Python). The language-pack move is a version *change*, so pip
///   uninstalls 1.8.1 via its RECORD (removing the old grammar `.so`s) before
///   unpacking 0.13.0 — no stale native extension is layered, unlike the
///   same-version in-place rebuilds this floor guards against. headroom-ai's own
///   abi3 wheel is likewise uninstalled-then-reinstalled. The 0.20.x+ cohort
///   upgrades in place. Raise the floor only if a future lock adds or
///   ABI-bumps a native transitive dep without a version bump.
pub(super) const ATOMIC_REBUILD_FLOOR_VERSION: (u32, u32, u32) = (0, 20, 0);

/// Parse the leading `major.minor.patch` from a version string, tolerating
/// pre-release/build suffixes (`-rc.1`, `+build`, `.dev0`, etc.). Returns
/// None when the prefix isn't a numeric `major.minor`. `patch` defaults to
/// 0 when missing or unparseable, so `"0.19"` and `"0.19.0"` compare equal.
pub(super) fn parse_major_minor_patch(s: &str) -> Option<(u32, u32, u32)> {
    let head = s.split(|c: char| c == '-' || c == '+').next()?;
    let mut parts = head.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

/// True when the previously-installed receipt is too old to safely apply an
/// in-place pip upgrade against — caller should fall through to the atomic
/// venv rebuild path. Unparseable versions are treated as too old (be
/// conservative: a rebuild is always safe, an unsafe in-place is not).
pub(super) fn receipt_requires_atomic_rebuild(previous_version: &str) -> bool {
    match parse_major_minor_patch(previous_version) {
        Some(v) => v < ATOMIC_REBUILD_FLOOR_VERSION,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_major_minor_patch, receipt_requires_atomic_rebuild, ATOMIC_REBUILD_FLOOR_VERSION,
    };

    #[test]
    fn parse_major_minor_patch_handles_clean_and_pre_release() {
        assert_eq!(parse_major_minor_patch("0.19.0"), Some((0, 19, 0)));
        assert_eq!(parse_major_minor_patch("1.2.3"), Some((1, 2, 3)));
        // Patch defaults to 0.
        assert_eq!(parse_major_minor_patch("0.19"), Some((0, 19, 0)));
        // Pre-release / build suffixes are stripped.
        assert_eq!(parse_major_minor_patch("0.19.0-rc.1"), Some((0, 19, 0)));
        assert_eq!(parse_major_minor_patch("0.19.0+build.5"), Some((0, 19, 0)));
        assert_eq!(parse_major_minor_patch("0.19.0.dev0"), Some((0, 19, 0)));
        // Nonsense returns None; caller treats it as "rebuild" to be safe.
        assert_eq!(parse_major_minor_patch(""), None);
        assert_eq!(parse_major_minor_patch("not-a-version"), None);
        assert_eq!(parse_major_minor_patch("0"), None);
    }

    #[test]
    fn receipt_requires_atomic_rebuild_below_floor() {
        // Floor raised to 0.20.0 in 0.4.0: upstream 0.20.x switched
        // headroom-ai to a maturin/Rust-native single-wheel build (upstream
        // #355). The 0.10.x-0.19.x cohort was built against the old
        // py3-none-any wheel with no `headroom_core` `.so`; an in-place
        // upgrade onto the new native wheel would layer a fresh extension
        // on top of stale transitive native pins, which is the exact
        // segfault-on-import pattern this floor exists to prevent.
        assert_eq!(ATOMIC_REBUILD_FLOOR_VERSION, (0, 20, 0));

        // Pre-floor: every desktop shipment up to and including 0.3.x
        // (which bundled headroom-ai 0.19.0). Both the original 0.8.2
        // fallback cohort and the 0.10.x -> 0.19.x cohort now fall here.
        assert!(receipt_requires_atomic_rebuild("0.5.18"));
        assert!(receipt_requires_atomic_rebuild("0.8.2"));
        assert!(receipt_requires_atomic_rebuild("0.9.7"));
        assert!(receipt_requires_atomic_rebuild("0.10.4"));
        assert!(receipt_requires_atomic_rebuild("0.10.12"));
        assert!(receipt_requires_atomic_rebuild("0.19.0"));

        // At-or-above the floor: in-place is allowed (0.20.x cohort + future).
        assert!(!receipt_requires_atomic_rebuild("0.20.0"));
        assert!(!receipt_requires_atomic_rebuild("0.21.39"));
        assert!(!receipt_requires_atomic_rebuild("1.0.0"));

        // Pre-release suffixes don't change the comparison.
        assert!(!receipt_requires_atomic_rebuild("0.20.0-rc.1"));
        assert!(receipt_requires_atomic_rebuild("0.19.99-rc.1"));

        // Unparseable receipts are treated as too-old (conservative).
        assert!(receipt_requires_atomic_rebuild(""));
        assert!(receipt_requires_atomic_rebuild("garbage"));
    }
}
