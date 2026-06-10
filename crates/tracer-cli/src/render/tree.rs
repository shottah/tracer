//! Call-trace tree rendering, Phalcon-style: calls and events interleaved in
//! execution order, with decoded names, values, gas, and revert reasons.

use super::{amount_display, short_addr};
use std::fmt::Write as _;
use tracer_core::{CallKind, Frame, FrameLog, TraceReport, format_units};

pub fn render(report: &TraceReport) -> String {
    let mut out = String::new();
    let tx = &report.tx;
    let _ = writeln!(out, "tx      {:?}", tx.hash);
    let _ = writeln!(
        out,
        "status  {}   block {}   gas used {}   type {}",
        if tx.status { "success" } else { "FAILED" },
        tx.block_number,
        tx.gas_used,
        tx.tx_type,
    );
    let _ = writeln!(
        out,
        "fee     {} {}   (gas price {} gwei)",
        format_units(
            alloy_primitives::U256::from(tx.gas_used as u128 * tx.effective_gas_price),
            18
        ),
        report.native_symbol,
        format_units(alloy_primitives::U256::from(tx.effective_gas_price), 9),
    );
    out.push('\n');
    if let Some(root) = &report.trace {
        frame_line(report, root, "", &mut out);
        children(report, root, "", &mut out);
    }
    out
}

fn frame_line(report: &TraceReport, f: &Frame, prefix: &str, out: &mut String) {
    let mut line = String::new();
    let _ = write!(line, "{}", f.kind.as_str());

    match f.kind {
        CallKind::Create | CallKind::Create2 => {
            let target = f.to.map(|a| short_addr(report, a)).unwrap_or_else(|| "?".into());
            let _ = write!(line, " {target}");
        }
        CallKind::SelfDestruct => {
            let beneficiary = f.to.map(|a| short_addr(report, a)).unwrap_or_else(|| "?".into());
            let _ = write!(line, " {} → {beneficiary}", short_addr(report, f.from));
        }
        _ => {
            let target = f.to.map(|a| short_addr(report, a)).unwrap_or_else(|| "?".into());
            let _ = write!(line, " {target}");
            if let Some(d) = &f.decoded {
                if let Some(name) = &d.name {
                    let params: Vec<String> = d
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, clip(&p.value)))
                        .collect();
                    let _ = write!(line, ".{name}({})", params.join(", "));
                } else if !d.selector.is_empty() && !f.input.is_empty() {
                    let _ = write!(line, " [{}]", d.selector);
                }
            }
        }
    }

    if !f.value.is_zero() {
        let _ = write!(line, "  {{{} {}}}", format_units(f.value, 18), report.native_symbol);
    }
    if f.gas_used > 0 {
        let _ = write!(line, "  gas={}", f.gas_used);
    }
    if let Some(err) = &f.error {
        let _ = write!(line, "  ✗ {err}");
        if let Some(reason) = &f.revert_reason {
            let _ = write!(line, ": {reason}");
        }
    }
    let _ = writeln!(out, "{prefix}{line}");
}

/// Children and logs interleaved by log `position`.
fn children(report: &TraceReport, f: &Frame, prefix: &str, out: &mut String) {
    enum Item<'a> {
        Frame(&'a Frame),
        Log(&'a FrameLog),
        Storage(String),
    }
    let mut items: Vec<Item> = Vec::new();
    let mut li = 0;
    for (ci, c) in f.children.iter().enumerate() {
        while li < f.logs.len() && f.logs[li].position.map(|p| p <= ci as u64).unwrap_or(false) {
            items.push(Item::Log(&f.logs[li]));
            li += 1;
        }
        items.push(Item::Frame(c));
    }
    while li < f.logs.len() {
        items.push(Item::Log(&f.logs[li]));
        li += 1;
    }
    for w in &f.storage_writes {
        items.push(Item::Storage(format!(
            "sstore {} ← {}{}",
            clip(&format!("{:?}", w.slot)),
            clip(&format!("{:?}", w.value)),
            w.previous.map(|p| format!(" (was {})", clip(&format!("{p:?}")))).unwrap_or_default(),
        )));
    }

    let count = items.len();
    for (i, item) in items.into_iter().enumerate() {
        let last = i + 1 == count;
        let branch = if last { "└─ " } else { "├─ " };
        let cont = if last { "   " } else { "│  " };
        match item {
            Item::Frame(c) => {
                frame_line(report, c, &format!("{prefix}{branch}"), out);
                children(report, c, &format!("{prefix}{cont}"), out);
            }
            Item::Log(l) => log_line(report, l, &format!("{prefix}{branch}"), out),
            Item::Storage(s) => {
                let _ = writeln!(out, "{prefix}{branch}{s}");
            }
        }
    }
}

fn log_line(report: &TraceReport, l: &FrameLog, prefix: &str, out: &mut String) {
    let mut line = String::from("emit ");
    match &l.decoded {
        Some(d) => {
            let params: Vec<String> =
                d.params.iter().map(|p| format!("{}: {}", p.name, clip(&p.value))).collect();
            let _ = write!(line, "{}({})", d.name, params.join(", "));
        }
        None => {
            let topic = l
                .topics
                .first()
                .map(|t| clip(&format!("{t:?}")))
                .unwrap_or_else(|| "anonymous".into());
            let _ = write!(line, "log {topic}");
        }
    }
    let _ = write!(line, "  @{}", short_addr(report, l.address));
    let _ = writeln!(out, "{prefix}{line}");
}

fn clip(s: &str) -> String {
    if s.len() > 46 { format!("{}…{}", &s[..24], &s[s.len() - 8..]) } else { s.to_string() }
}

/// One-line summary of the transfers (used under the tree).
pub fn render_transfers(report: &TraceReport) -> String {
    let mut out = String::new();
    if report.transfers.is_empty() {
        return out;
    }
    let _ = writeln!(out, "\ntransfers ({}):", report.transfers.len());
    for t in &report.transfers {
        let _ = writeln!(
            out,
            "  {:>3}. {} → {}  {}",
            t.order + 1,
            short_addr(report, t.from),
            short_addr(report, t.to),
            amount_display(report, &t.amount, &t.asset),
        );
    }
    out
}
