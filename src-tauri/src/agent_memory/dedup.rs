use std::collections::{BTreeMap, BTreeSet};

/// Approximate tokens deliberately uses the same stable 4-byte estimate as
/// Repo Intelligence. It is an estimate, never provider telemetry.
pub fn estimate_tokens(content: &str) -> u64 {
    std::cmp::max(1, (content.len() as u64).saturating_add(3) / 4)
}

pub fn duplicate_tokens(contents: &[String]) -> Vec<u64> {
    let sets: Vec<BTreeSet<String>> = contents
        .iter()
        .map(|content| {
            content
                .lines()
                .map(normalize_line)
                .filter(|line| !line.is_empty())
                .collect()
        })
        .collect();
    let mut owners: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, lines) in sets.iter().enumerate() {
        for line in lines {
            owners.entry(line.clone()).or_default().push(index);
        }
    }
    let mut totals = vec![0; contents.len()];
    for (line, indexes) in owners {
        if indexes.len() > 1 {
            let tokens = estimate_tokens(&line);
            for index in indexes {
                totals[index] += tokens;
            }
        }
    }
    totals
}

pub fn compacted_token_estimate(content: &str) -> u64 {
    let mut seen = BTreeSet::new();
    let mut normalized = String::new();
    let mut previous_blank = false;
    for line in content.lines() {
        let key = normalize_line(line);
        let blank = key.is_empty();
        if (blank && previous_blank) || (!blank && !seen.insert(key)) {
            continue;
        }
        normalized.push_str(line.trim_end());
        normalized.push('\n');
        previous_blank = blank;
    }
    estimate_tokens(&normalized)
}

pub fn normalize_line(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_instruction_is_counted_for_each_source() {
        let result = duplicate_tokens(&[
            "Always test changes.".into(),
            "always   test changes.".into(),
        ]);
        assert!(result.iter().all(|tokens| *tokens > 0));
    }
}
