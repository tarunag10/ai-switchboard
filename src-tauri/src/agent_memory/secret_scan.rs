use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretScanStatus {
    Clear,
    Blocked,
    Unreadable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretScanResult {
    pub status: SecretScanStatus,
    pub reason: Option<String>,
    pub finding_count: usize,
    pub categories: Vec<String>,
    pub affected_line_numbers: Vec<usize>,
}

pub fn scan(content: &str) -> SecretScanResult {
    let mut categories = Vec::new();
    let mut lines = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let mut category = None;
        if line.contains("-----BEGIN ") && line.contains("PRIVATE KEY-----") {
            category = Some("private_key");
        } else if line.contains("AKIA") && line.len() >= 20 {
            category = Some("aws_access_key");
        } else if line.contains("github_pat_") || line.contains("ghp_") {
            category = Some("github_token");
        } else if line.contains("sk-") && line.len() >= 24 {
            category = Some("api_key");
        } else if (lower.contains("api_key")
            || lower.contains("api-key")
            || lower.contains("access_token")
            || lower.contains("password")
            || lower.contains("secret"))
            && has_non_placeholder_assignment(line)
        {
            category = Some("credential_assignment");
        }
        if let Some(category) = category {
            if !categories.iter().any(|existing| existing == category) {
                categories.push(category.to_string());
            }
            if lines.len() < 20 {
                lines.push(index + 1);
            }
        }
    }
    let finding_count = lines.len();
    SecretScanResult {
        status: if finding_count == 0 {
            SecretScanStatus::Clear
        } else {
            SecretScanStatus::Blocked
        },
        reason: if finding_count == 0 {
            None
        } else {
            Some("Potential credential material was found; content is withheld.".to_string())
        },
        finding_count,
        categories,
        affected_line_numbers: lines,
    }
}

fn has_non_placeholder_assignment(line: &str) -> bool {
    let Some((_, value)) = line.split_once(['=', ':']) else {
        return false;
    };
    let value = value.trim().trim_matches(['\'', '"']);
    !value.is_empty()
        && ![
            "example",
            "placeholder",
            "redacted",
            "your_",
            "<",
            "${",
            "$ENV",
        ]
        .iter()
        .any(|marker| value.to_ascii_lowercase().contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_realistic_secret_patterns_without_returning_values() {
        let scan = scan("TOKEN=sk-abcdefghijklmnopqrstuvwxyz\n");
        assert!(matches!(scan.status, SecretScanStatus::Blocked));
        assert_eq!(scan.categories, vec!["api_key"]);
        assert_eq!(scan.affected_line_numbers, vec![1]);
    }

    #[test]
    fn permits_documented_placeholders() {
        let scan = scan("API_KEY=your_api_key_here\n");
        assert!(matches!(scan.status, SecretScanStatus::Clear));
    }
}
