//! Bounded, authority-free parsing for supplied Codex npm manifest bytes.

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::fmt;

pub(super) const MAX_CODEX_NPM_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_JSON_DEPTH: usize = 16;
const MAX_JSON_STRING_BYTES: usize = 1024;
const MAX_SEMVER_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmRootManifest {
    pub name: String,
    pub version: String,
    pub bin_codex: String,
    pub host_dependency_alias: String,
    pub host_dependency_spec: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmPlatformManifest {
    pub name: String,
    pub version: String,
    pub os: String,
    pub cpu: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmPayloadManifest {
    pub layout_version: u64,
    pub version: String,
    pub target: String,
    pub variant: String,
    pub entrypoint: String,
    pub resources_dir: String,
    pub path_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmManifestError {
    InputTooLarge,
    InvalidUtf8,
    ExcessiveNesting,
    OverlongString,
    InvalidJsonOrSchema,
    InvalidVersion,
    InvalidHostAlias,
    MissingHostAlias,
    DuplicateHostAlias,
}

#[derive(Deserialize)]
struct RootWire {
    name: String,
    version: String,
    bin: BinWire,
    #[serde(rename = "optionalDependencies")]
    optional_dependencies: DependencyEntries,
}

#[derive(Deserialize)]
struct BinWire {
    codex: String,
}

#[derive(Deserialize)]
struct PlatformWire {
    name: String,
    version: String,
    os: Vec<String>,
    cpu: Vec<String>,
}

#[derive(Deserialize)]
struct PayloadWire {
    #[serde(rename = "layoutVersion")]
    layout_version: u64,
    version: String,
    target: String,
    variant: String,
    entrypoint: String,
    #[serde(rename = "resourcesDir")]
    resources_dir: String,
    #[serde(rename = "pathDir")]
    path_dir: String,
}

struct DependencyEntries(Vec<(String, String)>);

impl<'de> Deserialize<'de> for DependencyEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = DependencyEntries;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an object mapping npm aliases to version specifications")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0).min(64));
                while let Some(entry) = map.next_entry::<String, String>()? {
                    entries.push(entry);
                }
                Ok(DependencyEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

pub(super) fn parse_codex_npm_root_manifest(
    input: &[u8],
    host_alias: &str,
) -> Result<CodexNpmRootManifest, CodexNpmManifestError> {
    if host_alias.is_empty() || host_alias.len() > MAX_JSON_STRING_BYTES {
        return Err(CodexNpmManifestError::InvalidHostAlias);
    }
    let wire: RootWire = parse_json(input)?;
    require_codex_npm_semver(&wire.version)?;

    let mut selected = None;
    for (alias, specification) in wire.optional_dependencies.0 {
        if alias == host_alias {
            if selected.replace(specification).is_some() {
                return Err(CodexNpmManifestError::DuplicateHostAlias);
            }
        }
    }
    let host_dependency_spec = selected.ok_or(CodexNpmManifestError::MissingHostAlias)?;
    Ok(CodexNpmRootManifest {
        name: wire.name,
        version: wire.version,
        bin_codex: wire.bin.codex,
        host_dependency_alias: host_alias.to_owned(),
        host_dependency_spec,
    })
}

pub(super) fn parse_codex_npm_platform_manifest(
    input: &[u8],
) -> Result<CodexNpmPlatformManifest, CodexNpmManifestError> {
    let wire: PlatformWire = parse_json(input)?;
    require_codex_npm_semver(&wire.version)?;
    let os = only_element(wire.os)?;
    let cpu = only_element(wire.cpu)?;
    Ok(CodexNpmPlatformManifest {
        name: wire.name,
        version: wire.version,
        os,
        cpu,
    })
}

pub(super) fn parse_codex_npm_payload_manifest(
    input: &[u8],
) -> Result<CodexNpmPayloadManifest, CodexNpmManifestError> {
    let wire: PayloadWire = parse_json(input)?;
    require_codex_npm_semver(&wire.version)?;
    Ok(CodexNpmPayloadManifest {
        layout_version: wire.layout_version,
        version: wire.version,
        target: wire.target,
        variant: wire.variant,
        entrypoint: wire.entrypoint,
        resources_dir: wire.resources_dir,
        path_dir: wire.path_dir,
    })
}

fn parse_json<T>(input: &[u8]) -> Result<T, CodexNpmManifestError>
where
    T: for<'de> Deserialize<'de>,
{
    validate_input(input)?;
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = T::deserialize(&mut deserializer)
        .map_err(|_| CodexNpmManifestError::InvalidJsonOrSchema)?;
    deserializer
        .end()
        .map_err(|_| CodexNpmManifestError::InvalidJsonOrSchema)?;
    Ok(value)
}

fn validate_input(input: &[u8]) -> Result<(), CodexNpmManifestError> {
    if input.len() > MAX_CODEX_NPM_MANIFEST_BYTES {
        return Err(CodexNpmManifestError::InputTooLarge);
    }
    let text = std::str::from_utf8(input).map_err(|_| CodexNpmManifestError::InvalidUtf8)?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut string_bytes = 0usize;

    for byte in text.bytes() {
        if in_string {
            if byte == b'"' && !escaped {
                in_string = false;
                continue;
            }
            string_bytes += 1;
            if string_bytes > MAX_JSON_STRING_BYTES {
                return Err(CodexNpmManifestError::OverlongString);
            }
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            }
            continue;
        }
        match byte {
            b'"' => {
                in_string = true;
                string_bytes = 0;
            }
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_JSON_DEPTH {
                    return Err(CodexNpmManifestError::ExcessiveNesting);
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn require_codex_npm_semver(value: &str) -> Result<(), CodexNpmManifestError> {
    is_codex_npm_semver(value)
        .then_some(())
        .ok_or(CodexNpmManifestError::InvalidVersion)
}

pub(super) fn is_codex_npm_semver(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_SEMVER_BYTES || value.contains('+') {
        return false;
    }
    let (core, prerelease) = match value.split_once('-') {
        Some((core, prerelease)) => (core, Some(prerelease)),
        None => (value, None),
    };
    let components = core.split('.').collect::<Vec<_>>();
    components.len() == 3
        && components.into_iter().all(valid_numeric_identifier)
        && prerelease.is_none_or(|value| {
            !value.is_empty()
                && value.split('.').all(|identifier| {
                    !identifier.is_empty()
                        && identifier
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                        && (!identifier.bytes().all(|byte| byte.is_ascii_digit())
                            || valid_numeric_identifier(identifier))
                })
        })
}

fn valid_numeric_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn only_element(mut values: Vec<String>) -> Result<String, CodexNpmManifestError> {
    if values.len() != 1 {
        return Err(CodexNpmManifestError::InvalidJsonOrSchema);
    }
    Ok(values.remove(0))
}
