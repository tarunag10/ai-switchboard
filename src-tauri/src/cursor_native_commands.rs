use serde::Serialize;

use crate::client_paths;
use crate::cursor_native::{assess_native_schema, evidence_lines};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorNativeSchemaAssessmentPublic {
    pub schema_id: String,
    pub supported: bool,
    pub reason: String,
    pub docs_url: String,
    pub surfaces_detected: usize,
    pub evidence: Vec<String>,
}

/// Returns the Cursor native-provider schema assessment used to gate writes.
#[tauri::command]
pub fn get_cursor_native_schema_assessment() -> CursorNativeSchemaAssessmentPublic {
    let assessment = assess_native_schema(&client_paths::home_dir());
    CursorNativeSchemaAssessmentPublic {
        schema_id: assessment.schema_id.to_string(),
        supported: assessment.supported,
        reason: assessment.reason.to_string(),
        docs_url: assessment.docs_url.to_string(),
        surfaces_detected: assessment.surfaces.len(),
        evidence: evidence_lines(&assessment),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assessment_keeps_native_writes_blocked() {
        let assessment = get_cursor_native_schema_assessment();
        assert!(!assessment.supported);
        assert_eq!(assessment.schema_id, "cursor-native-provider-schema");
        assert!(assessment.evidence.iter().any(|line| line.contains("remain blocked")));
    }
}
