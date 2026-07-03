use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RtkFramework {
    Vitest,
    Jest,
    Pytest,
    Cargo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RtkPreset {
    pub(crate) framework: RtkFramework,
    pub(crate) command_prefix: String,
    pub(crate) noisy_patterns: Vec<String>,
    pub(crate) keep_patterns: Vec<String>,
}

pub(crate) fn preset_for_framework(framework: RtkFramework) -> RtkPreset {
    match framework {
        RtkFramework::Vitest => RtkPreset {
            framework,
            command_prefix: "rtk test npm run test".to_string(),
            noisy_patterns: vec!["stdout |".to_string(), "stderr |".to_string()],
            keep_patterns: vec!["FAIL".to_string(), "Error:".to_string()],
        },
        RtkFramework::Jest => RtkPreset {
            framework,
            command_prefix: "rtk test npm run jest".to_string(),
            noisy_patterns: vec!["Snapshots:".to_string(), "Time:".to_string()],
            keep_patterns: vec!["FAIL".to_string(), "Expected:".to_string()],
        },
        RtkFramework::Pytest => RtkPreset {
            framework,
            command_prefix: "rtk pytest".to_string(),
            noisy_patterns: vec!["collected ".to_string(), "warnings summary".to_string()],
            keep_patterns: vec!["FAILED".to_string(), "E   ".to_string()],
        },
        RtkFramework::Cargo => RtkPreset {
            framework,
            command_prefix: "rtk cargo test".to_string(),
            noisy_patterns: vec!["Compiling ".to_string(), "Finished ".to_string()],
            keep_patterns: vec!["failures:".to_string(), "panicked at".to_string()],
        },
    }
}

pub(crate) fn all_presets() -> Vec<RtkPreset> {
    vec![
        preset_for_framework(RtkFramework::Vitest),
        preset_for_framework(RtkFramework::Jest),
        preset_for_framework(RtkFramework::Pytest),
        preset_for_framework(RtkFramework::Cargo),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_all_required_framework_presets() {
        let presets = all_presets();

        assert_eq!(presets.len(), 4);
        assert!(presets
            .iter()
            .any(|preset| preset.framework == RtkFramework::Cargo));
    }
}
