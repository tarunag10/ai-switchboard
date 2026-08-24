//! Pure executable candidate planning for platform adapters.
//!
//! This module does not read environment variables, inspect the filesystem, or
//! start processes. Callers inject PATH entries and PATHEXT text, then decide
//! how (or whether) to inspect the returned candidates.

use std::fmt;
use std::path::PathBuf;

pub const DEFAULT_WINDOWS_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";
pub const MAX_EXECUTABLE_CANDIDATES: usize = 16_384;
const MAX_WINDOWS_PATHEXT_BYTES: usize = 4_096;
const MAX_WINDOWS_PATHEXT_ENTRIES: usize = 64;
const MAX_BINARY_NAME_BYTES: usize = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableSearchPlatform {
    Unix,
    Windows,
    Unsupported,
}

impl ExecutableSearchPlatform {
    pub const fn current() -> Self {
        #[cfg(unix)]
        {
            Self::Unix
        }
        #[cfg(windows)]
        {
            Self::Windows
        }
        #[cfg(not(any(unix, windows)))]
        {
            Self::Unsupported
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableSearchError {
    UnsupportedPlatform,
    InvalidBinaryName,
    InvalidWindowsPathExtension,
    WindowsPathExtensionsTooLarge,
    TooManyCandidates,
}

impl fmt::Display for ExecutableSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedPlatform => "executable search platform is unsupported",
            Self::InvalidBinaryName => "executable search binary name is invalid",
            Self::InvalidWindowsPathExtension => {
                "executable search Windows path extension is invalid"
            }
            Self::WindowsPathExtensionsTooLarge => {
                "executable search Windows path extensions exceed the limit"
            }
            Self::TooManyCandidates => "executable search candidate count exceeds the limit",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ExecutableSearchError {}

pub fn plan_executable_candidates(
    platform: ExecutableSearchPlatform,
    directories: &[PathBuf],
    binary_names: &[&str],
    windows_pathext: Option<&str>,
) -> Result<Vec<PathBuf>, ExecutableSearchError> {
    if platform == ExecutableSearchPlatform::Unsupported {
        return Err(ExecutableSearchError::UnsupportedPlatform);
    }

    if binary_names.iter().any(|name| !valid_binary_name(name)) {
        return Err(ExecutableSearchError::InvalidBinaryName);
    }

    let extensions = match platform {
        ExecutableSearchPlatform::Unix => Vec::new(),
        ExecutableSearchPlatform::Windows => windows_path_extensions(windows_pathext)?,
        ExecutableSearchPlatform::Unsupported => unreachable!("handled above"),
    };
    let variants_per_name = 1usize
        .checked_add(extensions.len())
        .ok_or(ExecutableSearchError::TooManyCandidates)?;
    let candidate_count = directories
        .len()
        .checked_mul(binary_names.len())
        .and_then(|count| count.checked_mul(variants_per_name))
        .ok_or(ExecutableSearchError::TooManyCandidates)?;
    if candidate_count > MAX_EXECUTABLE_CANDIDATES {
        return Err(ExecutableSearchError::TooManyCandidates);
    }

    let mut candidates = Vec::with_capacity(candidate_count);
    for directory in directories {
        for binary_name in binary_names {
            candidates.push(directory.join(binary_name));
            for extension in &extensions {
                candidates.push(directory.join(format!("{binary_name}{extension}")));
            }
        }
    }
    Ok(candidates)
}

fn valid_binary_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_BINARY_NAME_BYTES
        && !matches!(name, "." | "..")
        && !name
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\' | ':'))
}

fn windows_path_extensions(
    windows_pathext: Option<&str>,
) -> Result<Vec<String>, ExecutableSearchError> {
    let value = windows_pathext.unwrap_or(DEFAULT_WINDOWS_PATHEXT);
    if value.len() > MAX_WINDOWS_PATHEXT_BYTES {
        return Err(ExecutableSearchError::WindowsPathExtensionsTooLarge);
    }

    let mut extensions = Vec::new();
    for extension in value.split(';').filter(|extension| !extension.is_empty()) {
        if extensions.len() >= MAX_WINDOWS_PATHEXT_ENTRIES {
            return Err(ExecutableSearchError::WindowsPathExtensionsTooLarge);
        }
        let extension = if extension.starts_with('.') {
            extension.to_string()
        } else {
            format!(".{extension}")
        };
        if extension == "."
            || extension
                .chars()
                .any(|character| character.is_control() || matches!(character, '/' | '\\' | ':'))
        {
            return Err(ExecutableSearchError::InvalidWindowsPathExtension);
        }
        extensions.push(extension);
    }
    Ok(extensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_candidates_preserve_directory_and_name_order_without_extensions() {
        let directories = vec![PathBuf::from("/first/bin"), PathBuf::from("/second/bin")];
        let candidates = plan_executable_candidates(
            ExecutableSearchPlatform::Unix,
            &directories,
            &["claude", "codex"],
            Some(".CMD;.EXE"),
        )
        .expect("valid Unix search");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/first/bin/claude"),
                PathBuf::from("/first/bin/codex"),
                PathBuf::from("/second/bin/claude"),
                PathBuf::from("/second/bin/codex"),
            ]
        );
    }

    #[test]
    fn windows_candidates_preserve_bare_then_pathext_order() {
        let directories = vec![PathBuf::from("C:/first"), PathBuf::from("C:/second")];
        let candidates = plan_executable_candidates(
            ExecutableSearchPlatform::Windows,
            &directories,
            &["codex"],
            Some("CMD;.EXE;.cmd"),
        )
        .expect("valid Windows search");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("C:/first/codex"),
                PathBuf::from("C:/first/codex.CMD"),
                PathBuf::from("C:/first/codex.EXE"),
                PathBuf::from("C:/first/codex.cmd"),
                PathBuf::from("C:/second/codex"),
                PathBuf::from("C:/second/codex.CMD"),
                PathBuf::from("C:/second/codex.EXE"),
                PathBuf::from("C:/second/codex.cmd"),
            ]
        );
    }

    #[test]
    fn windows_candidates_use_existing_default_and_empty_override_behavior() {
        let directory = vec![PathBuf::from("C:/tools")];
        let defaulted = plan_executable_candidates(
            ExecutableSearchPlatform::Windows,
            &directory,
            &["codex"],
            None,
        )
        .expect("default PATHEXT");
        assert_eq!(
            defaulted,
            vec![
                PathBuf::from("C:/tools/codex"),
                PathBuf::from("C:/tools/codex.COM"),
                PathBuf::from("C:/tools/codex.EXE"),
                PathBuf::from("C:/tools/codex.BAT"),
                PathBuf::from("C:/tools/codex.CMD"),
            ]
        );

        let empty = plan_executable_candidates(
            ExecutableSearchPlatform::Windows,
            &directory,
            &["codex"],
            Some(""),
        )
        .expect("empty PATHEXT override");
        assert_eq!(empty, vec![PathBuf::from("C:/tools/codex")]);
    }

    #[test]
    fn invalid_names_extensions_and_platforms_fail_closed() {
        let directory = vec![PathBuf::from("/tools")];
        for name in ["", ".", "..", "nested/codex", "nested\\codex", "C:codex"] {
            assert_eq!(
                plan_executable_candidates(
                    ExecutableSearchPlatform::Unix,
                    &directory,
                    &[name],
                    None,
                ),
                Err(ExecutableSearchError::InvalidBinaryName)
            );
        }
        assert_eq!(
            plan_executable_candidates(
                ExecutableSearchPlatform::Windows,
                &directory,
                &["codex"],
                Some(".EXE;../CMD"),
            ),
            Err(ExecutableSearchError::InvalidWindowsPathExtension)
        );
        assert_eq!(
            plan_executable_candidates(
                ExecutableSearchPlatform::Unsupported,
                &directory,
                &["codex"],
                None,
            ),
            Err(ExecutableSearchError::UnsupportedPlatform)
        );
    }

    #[test]
    fn oversized_extension_and_candidate_sets_fail_before_expansion() {
        let too_many_extensions = (0..=MAX_WINDOWS_PATHEXT_ENTRIES)
            .map(|index| format!(".X{index}"))
            .collect::<Vec<_>>()
            .join(";");
        assert_eq!(
            plan_executable_candidates(
                ExecutableSearchPlatform::Windows,
                &[PathBuf::from("C:/tools")],
                &["codex"],
                Some(&too_many_extensions),
            ),
            Err(ExecutableSearchError::WindowsPathExtensionsTooLarge)
        );

        let directories = (0..=MAX_EXECUTABLE_CANDIDATES)
            .map(|index| PathBuf::from(format!("/path/{index}")))
            .collect::<Vec<_>>();
        assert_eq!(
            plan_executable_candidates(
                ExecutableSearchPlatform::Unix,
                &directories,
                &["codex"],
                None,
            ),
            Err(ExecutableSearchError::TooManyCandidates)
        );
    }

    #[test]
    fn planning_is_deterministic() {
        let directories = vec![PathBuf::from("C:/tools")];
        let first = plan_executable_candidates(
            ExecutableSearchPlatform::Windows,
            &directories,
            &["claude", "codex"],
            Some(".CMD;.EXE"),
        )
        .expect("first plan");
        let second = plan_executable_candidates(
            ExecutableSearchPlatform::Windows,
            &directories,
            &["claude", "codex"],
            Some(".CMD;.EXE"),
        )
        .expect("second plan");
        assert_eq!(first, second);
    }
}
