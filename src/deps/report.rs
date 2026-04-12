/// Report formatters for dependency graph analysis.
///
/// Provides table and JSON output showing per-file fan-in, fan-out,
/// coupling classification, and cycle membership. Cycles are printed
/// separately after the main table.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::report_helpers;

use super::analyzer::{DepEntry, DepResult, JsonDepResult};

const COL_LANG: usize = 10;
const COL_FAN_IN: usize = 6;
const COL_FAN_OUT: usize = 7;
const COL_CYCLE: usize = 5;
// spacing: 1 (lead) + 2 + 1 + 1 + 1 + 1 = 7
const FIXED_WIDTH: usize = 7 + COL_LANG + COL_FAN_IN + COL_FAN_OUT + COL_CYCLE;

/// Print a table of per-file dependency metrics and a cycle summary.
pub fn print_report(entries: &[DepEntry], result: &DepResult) {
    if entries.is_empty() {
        println!("No source files found for dependency analysis.");
        return;
    }

    let max_path = report_helpers::max_path_width(entries.iter().map(|e| e.path.as_path()), 4);
    let header_width = max_path + FIXED_WIDTH;
    let sep = report_helpers::separator(header_width.max(72));

    println!("Dependency Graph");
    println!("{sep}");
    println!(
        " {:<pw$}  {:>COL_LANG$} {:>COL_FAN_IN$} {:>COL_FAN_OUT$} {:>COL_CYCLE$}",
        "File",
        "Language",
        "Fan-In",
        "Fan-Out",
        "Cycle",
        pw = max_path,
    );
    println!("{sep}");

    for e in entries {
        println!(
            " {:<pw$}  {:>COL_LANG$} {:>COL_FAN_IN$} {:>COL_FAN_OUT$} {:>COL_CYCLE$}",
            e.path.display(),
            e.language,
            e.fan_in,
            e.fan_out,
            if e.in_cycle { "yes" } else { "no" },
            pw = max_path,
        );
    }

    println!("{sep}");

    if result.cycles.is_empty() {
        println!("No dependency cycles detected.");
    } else {
        println!();
        println!("Dependency cycles: {}", result.cycles.len());
        for (i, cycle) in result.cycles.iter().enumerate() {
            println!("  Cycle {} ({} files):", i + 1, cycle.len());
            for p in cycle {
                println!("    {}", p.display());
            }
        }
    }
}

/// Serialize dependency analysis as pretty-printed JSON to stdout.
pub fn print_json(result: &DepResult) -> Result<(), Box<dyn std::error::Error>> {
    let out = JsonDepResult::from(result);
    report_helpers::print_json_stdout(&out)
}

/// Export dependency graph in DOT format for Graphviz visualization.
///
/// Node colors:
///   - lightsalmon  = participates in a dependency cycle
///   - lightgreen   = stable leaf (fan_out=0, imported by others)
///   - lightblue    = default
///
/// Usage: km deps --format dot > deps.dot && dot -Tsvg deps.dot -o deps.svg
pub fn print_dot(result: &DepResult, edges: &HashMap<PathBuf, Vec<PathBuf>>) {
    print!("{}", build_dot(result, edges));
}

pub(crate) fn build_dot(result: &DepResult, edges: &HashMap<PathBuf, Vec<PathBuf>>) -> String {
    let entry_set: HashSet<&PathBuf> = result.entries.iter().map(|e| &e.path).collect();

    let mut out = String::new();
    out.push_str("digraph deps {\n");
    out.push_str("    rankdir=LR;\n");
    out.push_str("    node [fontname=\"Helvetica\", fontsize=10, shape=box];\n");
    out.push('\n');
    for entry in &result.entries {
        out.push_str(&dot_node_line(entry));
    }
    out.push('\n');
    for (src, dsts) in edges {
        out.push_str(&dot_edge_lines(src, dsts, &entry_set));
    }
    out.push('\n');
    out.push_str(dot_legend());
    out.push_str("}\n");
    out
}

fn node_fillcolor(entry: &DepEntry) -> &'static str {
    if entry.in_cycle {
        "lightsalmon"
    } else if entry.fan_out == 0 && entry.fan_in > 0 {
        "lightgreen"
    } else {
        "lightblue"
    }
}

fn dot_node_line(entry: &DepEntry) -> String {
    let label = entry
        .path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.path.display().to_string());
    let tooltip = format!(
        "{}\\nfi={} fo={}",
        entry.path.display(),
        entry.fan_in,
        entry.fan_out
    );
    format!(
        "    {id} [label=\"{label}\", fillcolor={color}, style=filled, tooltip=\"{tooltip}\"];\n",
        id = dot_id(&entry.path),
        color = node_fillcolor(entry),
    )
}

fn dot_edge_lines(src: &PathBuf, dsts: &[PathBuf], entry_set: &HashSet<&PathBuf>) -> String {
    if dsts.is_empty() || !entry_set.contains(src) {
        return String::new();
    }
    dsts.iter()
        .filter(|dst| entry_set.contains(dst))
        .map(|dst| format!("    {} -> {};\n", dot_id(src), dot_id(dst)))
        .collect()
}

fn dot_legend() -> &'static str {
    "    // Legend\n\
     \x20   subgraph cluster_legend {\n\
     \x20       label=\"Legend\"; style=dashed; fontsize=9;\n\
     \x20       l_cycle [label=\"in cycle\", fillcolor=lightsalmon, style=filled, shape=box, fontsize=9];\n\
     \x20       l_leaf  [label=\"stable leaf\", fillcolor=lightgreen, style=filled, shape=box, fontsize=9];\n\
     \x20       l_def   [label=\"default\", fillcolor=lightblue, style=filled, shape=box, fontsize=9];\n\
     \x20   }\n"
}

/// Produce a valid DOT node identifier from a file path (quoted string).
fn dot_id(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('"', "\\\""))
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
