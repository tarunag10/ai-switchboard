use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(crate) const PONYTAIL_SOURCE_COMMIT: &str = "2ed6c52c9d7e5e56942508591085fd45dea277d3";
pub(crate) const PONYTAIL_DISPLAY_VERSION: &str = "4.9.0";
const PONYTAIL_PACKAGE_JSON_SHA256: &str =
    "5f7e5ab891c31ae006b986d34e0977fda44fc54e654cc4e07531dd5dd6f96b8f";

const SOURCE_MANIFEST: &str = include_str!("../../third_party/ponytail/SOURCE.json");
const LICENSE: &str = include_str!("../../third_party/ponytail/LICENSE");
const CORE_SKILL: &str = include_str!("../../third_party/ponytail/skills/ponytail/SKILL.md");
const REVIEW_SKILL: &str =
    include_str!("../../third_party/ponytail/skills/ponytail-review/SKILL.md");
const AUDIT_SKILL: &str = include_str!("../../third_party/ponytail/skills/ponytail-audit/SKILL.md");
const DEBT_SKILL: &str = include_str!("../../third_party/ponytail/skills/ponytail-debt/SKILL.md");
const GAIN_SKILL: &str = include_str!("../../third_party/ponytail/skills/ponytail-gain/SKILL.md");
const HELP_SKILL: &str = include_str!("../../third_party/ponytail/skills/ponytail-help/SKILL.md");

const BUNDLED_FILES: [(&str, &str); 7] = [
    ("third_party/ponytail/LICENSE", LICENSE),
    ("third_party/ponytail/skills/ponytail/SKILL.md", CORE_SKILL),
    (
        "third_party/ponytail/skills/ponytail-review/SKILL.md",
        REVIEW_SKILL,
    ),
    (
        "third_party/ponytail/skills/ponytail-audit/SKILL.md",
        AUDIT_SKILL,
    ),
    (
        "third_party/ponytail/skills/ponytail-debt/SKILL.md",
        DEBT_SKILL,
    ),
    (
        "third_party/ponytail/skills/ponytail-gain/SKILL.md",
        GAIN_SKILL,
    ),
    (
        "third_party/ponytail/skills/ponytail-help/SKILL.md",
        HELP_SKILL,
    ),
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceManifest {
    commit: String,
    package_version: String,
    upstream_package_json_sha256: String,
    license: String,
    files: Vec<SourceFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceFile {
    local_path: String,
    sha256: String,
    modified: bool,
}

fn sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn verify_bundled_ponytail() -> Result<()> {
    let manifest: SourceManifest =
        serde_json::from_str(SOURCE_MANIFEST).context("decoding bundled Ponytail provenance")?;
    if manifest.commit != PONYTAIL_SOURCE_COMMIT
        || manifest.package_version != PONYTAIL_DISPLAY_VERSION
        || manifest.upstream_package_json_sha256 != PONYTAIL_PACKAGE_JSON_SHA256
        || manifest.license != "MIT"
    {
        bail!("bundled Ponytail provenance does not match this build");
    }
    if manifest.files.len() != BUNDLED_FILES.len() {
        bail!("bundled Ponytail provenance file count is incomplete");
    }
    for (path, content) in BUNDLED_FILES {
        let entry = manifest
            .files
            .iter()
            .find(|entry| entry.local_path == path)
            .with_context(|| format!("Ponytail provenance is missing {path}"))?;
        if entry.modified || entry.sha256 != sha256(content) {
            bail!("bundled Ponytail resource failed integrity validation: {path}");
        }
    }
    Ok(())
}

pub(crate) fn core_guidance() -> Result<&'static str> {
    verify_bundled_ponytail()?;
    let source = CORE_SKILL
        .strip_prefix("---\n")
        .context("bundled Ponytail core skill is missing frontmatter")?;
    let (_, body) = source
        .split_once("\n---\n")
        .context("bundled Ponytail core skill has malformed frontmatter")?;
    Ok(body.trim())
}

pub(crate) fn skill_ids() -> [&'static str; 6] {
    [
        "ponytail",
        "ponytail-review",
        "ponytail-audit",
        "ponytail-debt",
        "ponytail-gain",
        "ponytail-help",
    ]
}

#[cfg(test)]
mod tests {
    use super::{core_guidance, skill_ids, verify_bundled_ponytail};

    #[test]
    fn vendored_ponytail_resources_match_the_recorded_upstream_snapshot() {
        verify_bundled_ponytail().expect("Ponytail resources must match SOURCE.json");
        assert_eq!(skill_ids().len(), 6);
    }

    #[test]
    fn core_guidance_excludes_plugin_frontmatter() {
        let guidance = core_guidance().expect("valid core guidance");
        assert!(guidance.starts_with("# Ponytail"));
        assert!(!guidance.contains("argument-hint:"));
        assert!(guidance.contains("Never simplify away: input validation"));
    }
}
