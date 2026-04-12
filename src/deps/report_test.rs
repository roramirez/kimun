use super::*;
use crate::deps::analyzer::{DepEntry, DepResult};
use std::collections::HashMap;
use std::path::PathBuf;

fn make_entry(
    path: &str,
    language: &str,
    fan_in: usize,
    fan_out: usize,
    in_cycle: bool,
) -> DepEntry {
    DepEntry {
        path: PathBuf::from(path),
        language: language.to_string(),
        fan_in,
        fan_out,
        in_cycle,
    }
}

fn make_result(entries: Vec<DepEntry>, cycles: Vec<Vec<PathBuf>>) -> DepResult {
    DepResult { entries, cycles }
}

// ── print_report ────────────────────────────────────────────────────────────

#[test]
fn print_report_empty_entries() {
    let result = make_result(vec![], vec![]);
    print_report(&[], &result);
}

#[test]
fn print_report_no_cycles() {
    let entries = vec![
        make_entry("src/main.rs", "Rust", 0, 2, false),
        make_entry("src/lib.rs", "Rust", 2, 0, false),
    ];
    let result = make_result(
        entries
            .iter()
            .map(|e| {
                make_entry(
                    &e.path.display().to_string(),
                    &e.language,
                    e.fan_in,
                    e.fan_out,
                    e.in_cycle,
                )
            })
            .collect(),
        vec![],
    );
    print_report(&entries, &result);
}

#[test]
fn print_report_with_cycles() {
    let entries = vec![
        make_entry("src/a.rs", "Rust", 1, 1, true),
        make_entry("src/b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
    let result = make_result(
        entries
            .iter()
            .map(|e| {
                make_entry(
                    &e.path.display().to_string(),
                    &e.language,
                    e.fan_in,
                    e.fan_out,
                    e.in_cycle,
                )
            })
            .collect(),
        vec![cycle],
    );
    print_report(&entries, &result);
}

#[test]
fn print_report_mixed_cycle_and_clean() {
    let entries = vec![
        make_entry("src/clean.rs", "Rust", 0, 1, false),
        make_entry("src/cycle_a.rs", "Rust", 1, 1, true),
        make_entry("src/cycle_b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![
        PathBuf::from("src/cycle_a.rs"),
        PathBuf::from("src/cycle_b.rs"),
    ];
    let result = make_result(
        entries
            .iter()
            .map(|e| {
                make_entry(
                    &e.path.display().to_string(),
                    &e.language,
                    e.fan_in,
                    e.fan_out,
                    e.in_cycle,
                )
            })
            .collect(),
        vec![cycle],
    );
    print_report(&entries, &result);
}

// ── build_dot ────────────────────────────────────────────────────────────────

fn make_edges(pairs: &[(&str, &[&str])]) -> HashMap<PathBuf, Vec<PathBuf>> {
    pairs
        .iter()
        .map(|(k, vs)| {
            (
                PathBuf::from(k),
                vs.iter().map(|v| PathBuf::from(v)).collect(),
            )
        })
        .collect()
}

#[test]
fn build_dot_starts_with_digraph() {
    let result = make_result(vec![], vec![]);
    let edges = make_edges(&[]);
    let dot = super::build_dot(&result, &edges);
    assert!(dot.starts_with("digraph deps {"));
}

#[test]
fn build_dot_ends_with_closing_brace() {
    let result = make_result(vec![], vec![]);
    let edges = make_edges(&[]);
    let dot = super::build_dot(&result, &edges);
    assert!(dot.trim_end().ends_with('}'));
}

#[test]
fn build_dot_cycle_node_gets_lightsalmon() {
    let entries = vec![make_entry("src/a.rs", "Rust", 1, 1, true)];
    let result = make_result(entries, vec![]);
    let edges = make_edges(&[("src/a.rs", &[])]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        dot.contains("fillcolor=lightsalmon"),
        "cycle node must be lightsalmon"
    );
}

#[test]
fn build_dot_stable_leaf_gets_lightgreen() {
    // fan_out=0, fan_in>0 → stable leaf
    let entries = vec![make_entry("src/util.rs", "Rust", 3, 0, false)];
    let result = make_result(entries, vec![]);
    let edges = make_edges(&[("src/util.rs", &[])]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        dot.contains("fillcolor=lightgreen"),
        "stable leaf must be lightgreen"
    );
}

#[test]
fn build_dot_default_node_gets_lightblue() {
    let entries = vec![make_entry("src/main.rs", "Rust", 0, 2, false)];
    let result = make_result(entries, vec![]);
    let edges = make_edges(&[("src/main.rs", &[])]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        dot.contains("fillcolor=lightblue"),
        "default node must be lightblue"
    );
}

#[test]
fn build_dot_emits_edge_between_known_nodes() {
    let entries = vec![
        make_entry("src/main.rs", "Rust", 0, 1, false),
        make_entry("src/lib.rs", "Rust", 1, 0, false),
    ];
    let result = make_result(entries, vec![]);
    let edges = make_edges(&[("src/main.rs", &["src/lib.rs"]), ("src/lib.rs", &[])]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        dot.contains("\"src/main.rs\" -> \"src/lib.rs\""),
        "edge from main to lib must be present"
    );
}

#[test]
fn build_dot_skips_edge_to_unknown_node() {
    let entries = vec![make_entry("src/main.rs", "Rust", 0, 1, false)];
    let result = make_result(entries, vec![]);
    // "src/external.rs" is not in entries
    let edges = make_edges(&[("src/main.rs", &["src/external.rs"])]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        !dot.contains("external.rs"),
        "edges to unknown nodes must be omitted"
    );
}

#[test]
fn build_dot_includes_legend() {
    let result = make_result(vec![], vec![]);
    let edges = make_edges(&[]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        dot.contains("cluster_legend"),
        "legend subgraph must be present"
    );
    assert!(
        dot.contains("lightsalmon"),
        "legend must mention lightsalmon"
    );
    assert!(dot.contains("lightgreen"), "legend must mention lightgreen");
}

#[test]
fn build_dot_node_label_uses_filename_not_full_path() {
    let entries = vec![make_entry("src/foo/bar.rs", "Rust", 0, 0, false)];
    let result = make_result(entries, vec![]);
    let edges = make_edges(&[("src/foo/bar.rs", &[])]);
    let dot = super::build_dot(&result, &edges);
    assert!(
        dot.contains("label=\"bar.rs\""),
        "node label must be filename only, not full path"
    );
}

// ── print_json ───────────────────────────────────────────────────────────────

#[test]
fn print_json_empty() {
    let result = make_result(vec![], vec![]);
    print_json(&result).unwrap();
}

#[test]
fn print_json_with_entries_no_cycles() {
    let entries = vec![
        make_entry("src/main.rs", "Rust", 0, 3, false),
        make_entry("src/util.rs", "Rust", 3, 0, false),
    ];
    let result = make_result(entries, vec![]);
    print_json(&result).unwrap();
}

#[test]
fn print_json_with_cycles() {
    let entries = vec![
        make_entry("src/a.rs", "Rust", 1, 1, true),
        make_entry("src/b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
    let result = make_result(entries, vec![cycle]);
    print_json(&result).unwrap();
}
