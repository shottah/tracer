//! Fund-flow renderings: Mermaid (for docs/UIs) and DOT (for graphviz).

use super::{amount_display, short_addr};
use std::collections::HashMap;
use std::fmt::Write as _;
use tracer_core::TraceReport;

pub fn render_mermaid(report: &TraceReport) -> String {
    let Some(flow) = &report.fund_flow else {
        return "%% no fund flow computed\n".into();
    };
    let mut out = String::from("flowchart LR\n");
    let mut ids: HashMap<alloy_primitives::Address, String> = HashMap::new();
    for (i, node) in flow.nodes.iter().enumerate() {
        let id = format!("n{i}");
        let title = escape(&short_addr(report, node.id));
        let addr = format!("{:?}", node.id);
        let _ = writeln!(
            out,
            "    {id}[\"{title}<br/><small>{}…{}</small>\"]",
            &addr[..6],
            &addr[addr.len() - 4..]
        );
        ids.insert(node.id, id);
    }
    for edge in &flow.edges {
        let (Some(from), Some(to)) = (ids.get(&edge.from), ids.get(&edge.to)) else { continue };
        let label = escape(&format!(
            "{}: {}",
            edge.order + 1,
            amount_display(report, &edge.amount, &edge.asset)
        ));
        let _ = writeln!(out, "    {from} -->|\"{label}\"| {to}");
    }
    out
}

pub fn render_dot(report: &TraceReport) -> String {
    let Some(flow) = &report.fund_flow else {
        return "// no fund flow computed\n".into();
    };
    let mut out = String::from("digraph fundflow {\n    rankdir=LR;\n    node [shape=box];\n");
    for node in &flow.nodes {
        let _ = writeln!(
            out,
            "    \"{:?}\" [label=\"{}\"];",
            node.id,
            escape(&short_addr(report, node.id))
        );
    }
    for edge in &flow.edges {
        let _ = writeln!(
            out,
            "    \"{:?}\" -> \"{:?}\" [label=\"{}: {}\"];",
            edge.from,
            edge.to,
            edge.order + 1,
            escape(&amount_display(report, &edge.amount, &edge.asset))
        );
    }
    out.push_str("}\n");
    out
}

fn escape(s: &str) -> String {
    s.replace('"', "'").replace(['\n', '\r'], " ")
}
