//! Balance-changes table.

use super::{asset_label, short_addr};
use std::fmt::Write as _;
use tracer_core::TraceReport;

pub fn render(report: &TraceReport) -> String {
    let Some(bc) = &report.balance_changes else {
        return "no balance changes computed\n".into();
    };

    let mut rows: Vec<[String; 3]> = Vec::new();
    for change in &bc.changes {
        let who = match &change.label {
            Some(label) => format!("{} ({})", short_addr(report, change.address), label),
            None => short_addr(report, change.address),
        };
        let mut first = true;
        if let Some(n) = &change.native {
            let delta = n.formatted_or_dec();
            let extra = n
                .gas_fee
                .as_ref()
                .map(|g| {
                    format!(
                        "  (incl. gas {})",
                        g.formatted.clone().unwrap_or_else(|| g.dec.clone())
                    )
                })
                .unwrap_or_default();
            rows.push([who.clone(), report.native_symbol.clone(), format!("{delta}{extra}")]);
            first = false;
        }
        for tc in &change.tokens {
            let label = asset_label(report, &tc.asset);
            let delta = tc.delta.formatted.clone().unwrap_or_else(|| tc.delta.dec.clone());
            rows.push([if first { who.clone() } else { String::new() }, label, delta]);
            first = false;
        }
        let _ = first;
    }

    if rows.is_empty() {
        return "no balance changes\n".into();
    }

    let headers = ["account", "asset", "delta"];
    let mut widths = [headers[0].len(), headers[1].len(), headers[2].len()];
    for r in &rows {
        for (i, cell) in r.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<w0$}  {:<w1$}  {}",
        headers[0],
        headers[1],
        headers[2],
        w0 = widths[0],
        w1 = widths[1]
    );
    let _ = writeln!(out, "{}", "-".repeat(widths[0] + widths[1] + widths[2] + 4));
    for r in &rows {
        let _ =
            writeln!(out, "{:<w0$}  {:<w1$}  {}", r[0], r[1], r[2], w0 = widths[0], w1 = widths[1]);
    }
    let _ = writeln!(
        out,
        "\nnative source: {:?}   gas included: {}",
        bc.native_source, bc.gas_included
    );
    out
}

trait NativeDisplay {
    fn formatted_or_dec(&self) -> String;
}

impl NativeDisplay for tracer_core::NativeChange {
    fn formatted_or_dec(&self) -> String {
        self.delta.formatted.clone().unwrap_or_else(|| self.delta.dec.clone())
    }
}
