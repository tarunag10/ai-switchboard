use std::collections::BTreeSet;

use crate::models::{
    RepoContextPack, RepoContextPackRankingMetadata, RepoContextPackReverseDependencyEvidence,
    RepoFileRank, RepoFileRole, RepoFileSignal, RepoGraphEdgeKind, RepoGraphSummary,
};

use super::{MAX_PACK_FILES, TASK_PACK_BUDGET_TOKENS};

pub(super) fn build_context_pack(
    id: &str,
    title: &str,
    purpose: &str,
    files: Vec<RepoFileSignal>,
    estimated_full_scan_tokens: u64,
    graph: Option<&RepoGraphSummary>,
) -> RepoContextPack {
    let task_terms = task_terms(purpose);
    let mut ranked_files: Vec<(RepoFileSignal, RepoFileRank)> = files
        .into_iter()
        .map(|mut file| {
            let rank = rank_file_for_task(&file, &task_terms, purpose, graph);
            file.reasons
                .extend(rank.reasons.iter().map(|reason| format!("rank: {reason}")));
            (file, rank)
        })
        .collect();
    ranked_files.sort_by(|(a, rank_a), (b, rank_b)| {
        rank_b
            .score
            .partial_cmp(&rank_a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                score_per_token(&rank_b)
                    .partial_cmp(&score_per_token(&rank_a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.estimated_tokens.cmp(&b.estimated_tokens))
            .then_with(|| a.path.cmp(&b.path))
    });
    ranked_files.truncate(MAX_PACK_FILES);
    let ranking = build_ranking_metadata(&task_terms, &ranked_files, graph);
    let files = ranked_files
        .into_iter()
        .map(|(file, _)| file)
        .collect::<Vec<_>>();
    let estimated_tokens = files
        .iter()
        .map(|signal| signal.estimated_tokens)
        .sum::<u64>();
    let savings_vs_full_scan_pct = if estimated_full_scan_tokens > 0 {
        let saved: f64 = 1.0 - (estimated_tokens as f64 / estimated_full_scan_tokens as f64);
        (saved.max(0.0) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    RepoContextPack {
        id: id.to_string(),
        title: title.to_string(),
        purpose: purpose.to_string(),
        files,
        estimated_tokens,
        savings_vs_full_scan_pct,
        ranking,
    }
}

fn task_terms(task: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    task.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|term| {
            let term = term.trim().to_ascii_lowercase();
            if term.len() >= 3 && seen.insert(term.clone()) {
                Some(term)
            } else {
                None
            }
        })
        .take(8)
        .collect()
}

fn build_ranking_metadata(
    task_terms: &[String],
    ranked_files: &[(RepoFileSignal, RepoFileRank)],
    graph: Option<&RepoGraphSummary>,
) -> RepoContextPackRankingMetadata {
    let graph_task_term_match_count = ranked_files
        .iter()
        .filter(|(_, rank)| {
            rank.reasons
                .iter()
                .any(|reason| reason.starts_with("graph: ") && reason.contains("task"))
        })
        .count();
    let reverse_dependency_hubs = graph
        .map(|graph| {
            ranked_files
                .iter()
                .filter_map(|(file, _)| {
                    graph
                        .reverse_dependency_hubs
                        .iter()
                        .find(|hub| hub.label == file.path && hub.count > 0)
                        .map(|hub| RepoContextPackReverseDependencyEvidence {
                            path: hub.label.clone(),
                            incoming_references: hub.count,
                        })
                })
                .take(3)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut evidence = Vec::new();
    if !task_terms.is_empty() {
        evidence.push(format!("task terms: {}", task_terms.join(", ")));
    }
    if graph_task_term_match_count > 0 {
        evidence.push(format!(
            "graph task matches: {graph_task_term_match_count} selected file{}",
            if graph_task_term_match_count == 1 {
                ""
            } else {
                "s"
            }
        ));
    }
    if !reverse_dependency_hubs.is_empty() {
        evidence.push(format!(
            "reverse dependency hubs: {}",
            reverse_dependency_hubs
                .iter()
                .map(|hub| format!("{} ({})", hub.path, hub.incoming_references))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    RepoContextPackRankingMetadata {
        task_terms: task_terms.to_vec(),
        graph_task_term_match_count,
        reverse_dependency_hubs,
        evidence,
    }
}

fn score_per_token(rank: &RepoFileRank) -> f64 {
    rank.score / rank.estimated_tokens.max(1) as f64
}

#[derive(Debug, Default)]
struct RepoTaskGraphAffinity {
    score: f64,
    reasons: Vec<String>,
}

fn rank_file_for_task(
    file: &RepoFileSignal,
    task_terms: &[String],
    task: &str,
    graph: Option<&RepoGraphSummary>,
) -> RepoFileRank {
    let mut score = 0.0;
    let mut reasons = Vec::new();
    let mut risks = Vec::new();
    let lower_path = file.path.to_ascii_lowercase();
    let file_name = lower_path.rsplit('/').next().unwrap_or(&lower_path);

    match file.role {
        RepoFileRole::Source => {
            score += 50.0;
            reasons.push("source file".to_string());
        }
        RepoFileRole::Test => {
            score += if task.contains("Verification") || task.contains("Risk") {
                45.0
            } else {
                28.0
            };
            reasons.push("test proximity".to_string());
        }
        RepoFileRole::Config => {
            score += 35.0;
            reasons.push("project configuration".to_string());
        }
        RepoFileRole::Docs => {
            score += if task.contains("Handoff") || task.contains("Release") {
                34.0
            } else {
                14.0
            };
            reasons.push("documentation context".to_string());
        }
        RepoFileRole::Lockfile => {
            score += 8.0;
            risks.push("lockfile token cost can dominate packs".to_string());
        }
        RepoFileRole::Asset | RepoFileRole::Generated => {
            score -= 100.0;
            risks.push("generated or binary-like path".to_string());
        }
        RepoFileRole::Unknown => {
            score += 2.0;
        }
    }

    if matches!(
        file_name,
        "main.rs"
            | "lib.rs"
            | "main.ts"
            | "main.tsx"
            | "index.ts"
            | "index.tsx"
            | "app.ts"
            | "app.tsx"
            | "package.json"
            | "cargo.toml"
            | "pyproject.toml"
    ) {
        score += 30.0;
        reasons.push("entrypoint or project hub".to_string());
    }
    if lower_path.contains("/src/") || lower_path.starts_with("src/") {
        score += 8.0;
        reasons.push("source tree centrality".to_string());
    }
    if lower_path.contains("/test")
        || lower_path.contains(".test.")
        || lower_path.contains(".spec.")
    {
        score += 8.0;
        reasons.push("nearest tests candidate".to_string());
    }
    for term in task_terms {
        if lower_path.contains(term) {
            score += 16.0;
            reasons.push(format!("matches task term `{term}`"));
        }
    }
    let graph_affinity = task_graph_affinity(file, task_terms, graph);
    if graph_affinity.score > 0.0 {
        score += graph_affinity.score;
        reasons.extend(graph_affinity.reasons);
    }
    let reverse_dependency_importance = reverse_dependency_importance(file, graph);
    if reverse_dependency_importance.score > 0.0 {
        score += reverse_dependency_importance.score;
        reasons.extend(reverse_dependency_importance.reasons);
    }

    let token_penalty = (file.estimated_tokens as f64 / TASK_PACK_BUDGET_TOKENS as f64) * 12.0;
    score -= token_penalty.min(24.0);
    if file.estimated_tokens > TASK_PACK_BUDGET_TOKENS / 2 {
        risks.push("large token footprint".to_string());
    }

    RepoFileRank {
        path: file.path.clone(),
        score: (score * 10.0).round() / 10.0,
        estimated_tokens: file.estimated_tokens,
        reasons,
        risks,
    }
}

fn task_graph_affinity(
    file: &RepoFileSignal,
    task_terms: &[String],
    graph: Option<&RepoGraphSummary>,
) -> RepoTaskGraphAffinity {
    if task_terms.is_empty() {
        return RepoTaskGraphAffinity::default();
    }
    let Some(graph) = graph else {
        return RepoTaskGraphAffinity::default();
    };

    let mut affinity = RepoTaskGraphAffinity::default();
    let mut seen_reasons = BTreeSet::new();
    for symbol in &graph.symbols {
        if symbol.file != file.path {
            continue;
        }
        let symbol_key = format!(
            "{} {} {}",
            symbol.name,
            symbol.parent.as_deref().unwrap_or_default(),
            symbol.file
        )
        .to_ascii_lowercase();
        if let Some(term) = task_terms
            .iter()
            .find(|term| symbol_key.contains(term.as_str()))
        {
            affinity.score += 18.0;
            push_unique_affinity_reason(
                &mut affinity.reasons,
                &mut seen_reasons,
                format!("graph: indexed symbol matches task term `{term}`"),
            );
        }
    }

    for edge in graph.import_edges.iter().chain(graph.symbol_edges.iter()) {
        let edge_to_path = graph_edge_file_path(&edge.to);
        let counterpart = if edge.from == file.path {
            Some(edge_to_path)
        } else if edge_to_path == file.path {
            Some(edge.from.as_str())
        } else {
            None
        };
        let Some(counterpart) = counterpart else {
            continue;
        };
        let counterpart_lower = counterpart.to_ascii_lowercase();
        if task_terms
            .iter()
            .any(|term| counterpart_lower.contains(term.as_str()))
        {
            affinity.score += match edge.kind {
                RepoGraphEdgeKind::TestToSource | RepoGraphEdgeKind::ImportReference => 16.0,
                RepoGraphEdgeKind::CallReference | RepoGraphEdgeKind::SymbolReference => 14.0,
                RepoGraphEdgeKind::EntrypointToConfig
                | RepoGraphEdgeKind::SourceToDependencyHub
                | RepoGraphEdgeKind::PackageDependency => 10.0,
            };
            push_unique_affinity_reason(
                &mut affinity.reasons,
                &mut seen_reasons,
                format!(
                    "graph: connected to task-matching `{counterpart}` via {:?}",
                    edge.kind
                ),
            );
        }
    }
    affinity.score = affinity.score.min(48.0);
    affinity
}

fn reverse_dependency_importance(
    file: &RepoFileSignal,
    graph: Option<&RepoGraphSummary>,
) -> RepoTaskGraphAffinity {
    let Some(graph) = graph else {
        return RepoTaskGraphAffinity::default();
    };
    let Some(hub) = graph
        .reverse_dependency_hubs
        .iter()
        .find(|hub| hub.label == file.path)
    else {
        return RepoTaskGraphAffinity::default();
    };
    if hub.count == 0 {
        return RepoTaskGraphAffinity::default();
    }

    let mut importance = RepoTaskGraphAffinity::default();
    importance.score = (8.0 + (hub.count as f64 * 4.0)).min(28.0);
    let examples = hub.examples.iter().take(2).cloned().collect::<Vec<_>>();
    importance.reasons.push(if examples.is_empty() {
        format!(
            "graph: reverse dependency hub with {} incoming reference{}",
            hub.count,
            if hub.count == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "graph: reverse dependency hub with {} incoming reference{} from {}",
            hub.count,
            if hub.count == 1 { "" } else { "s" },
            examples.join(", ")
        )
    });
    importance
}

fn graph_edge_file_path(target: &str) -> &str {
    target
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(target)
}

fn push_unique_affinity_reason(
    reasons: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    reason: String,
) {
    if reasons.len() >= 3 {
        return;
    }
    if seen.insert(reason.clone()) {
        reasons.push(reason);
    }
}
