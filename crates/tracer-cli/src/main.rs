//! `tracer` — headless EVM transaction inspector.
//!
//! Open-source alternative to Phalcon Explorer / Tenderly transaction views:
//! call traces, balance changes, and fund flows, from any RPC (with a local
//! anvil-fork fallback when the endpoint has no `debug` API).

mod render;

use alloy_primitives::{Address, B256};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use tracer_client::{BackendChoice, ClientError, Tracer, TracerConfig};
use tracer_core::TraceReport;

#[derive(Parser)]
#[command(
    name = "tracer",
    version,
    about = "EVM transaction inspector: call traces, balance changes, fund flows",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the call trace of a transaction.
    Trace {
        #[command(flatten)]
        common: Common,
        #[arg(long, value_enum, default_value_t = TraceFormat::Tree)]
        format: TraceFormat,
    },
    /// Show per-account native and token balance changes.
    BalanceChanges {
        #[command(flatten)]
        common: Common,
        #[arg(long, value_enum, default_value_t = TableFormat::Table)]
        format: TableFormat,
    },
    /// Show the fund-flow transfer graph.
    FundFlow {
        #[command(flatten)]
        common: Common,
        #[arg(long, value_enum, default_value_t = FlowFormat::Mermaid)]
        format: FlowFormat,
    },
    /// Emit the full report envelope as JSON (everything in one document).
    Report {
        #[command(flatten)]
        common: Common,
        /// Compact (single-line) JSON instead of pretty-printed.
        #[arg(long)]
        compact: bool,
    },
}

#[derive(Args)]
struct Common {
    /// Transaction hash to inspect.
    tx_hash: B256,

    /// Ethereum JSON-RPC endpoint.
    #[arg(short, long, env = "ETH_RPC_URL")]
    rpc_url: String,

    /// How to obtain traces.
    #[arg(long, value_enum, default_value_t = Backend::Auto)]
    backend: Backend,

    /// Also run the struct-log interpreter: per-frame storage reads/writes
    /// and exact call/event interleaving (heavier).
    #[arg(long)]
    deep: bool,

    /// In fork mode, skip replaying the block's preceding transactions
    /// (faster, state may differ).
    #[arg(long)]
    no_replay: bool,

    /// Skip token metadata lookups and amount formatting.
    #[arg(long)]
    no_enrich: bool,

    /// Exclude gas fees from derived balance changes.
    #[arg(long)]
    no_gas: bool,

    /// Override the canonical wrapped-native token address.
    #[arg(long)]
    wrapped_native: Option<Address>,

    /// Write output to a file instead of stdout.
    #[arg(short, long)]
    output: Option<std::path::PathBuf>,

    /// Suppress warnings on stderr.
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Backend {
    Auto,
    Rpc,
    AnvilFork,
}

#[derive(Clone, Copy, ValueEnum)]
enum TraceFormat {
    Tree,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum TableFormat {
    Table,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum FlowFormat {
    Mermaid,
    Dot,
    Json,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let code = match run(cli).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e:#}");
            match e.downcast_ref::<ClientError>() {
                Some(ClientError::TxNotFound(_) | ClientError::NotMined(_)) => 2,
                _ => 1,
            }
        }
    };
    std::process::exit(code);
}

async fn run(cli: Cli) -> Result<()> {
    let (common, output) = match &cli.command {
        Command::Trace { common, format } => {
            let report = fetch(common).await?;
            let text = match format {
                TraceFormat::Tree => {
                    let mut s = render::tree::render(&report);
                    s.push_str(&render::tree::render_transfers(&report));
                    s
                }
                TraceFormat::Json => pretty(&projected(&report, Projection::Trace))?,
            };
            (common, text)
        }
        Command::BalanceChanges { common, format } => {
            let report = fetch(common).await?;
            let text = match format {
                TableFormat::Table => render::table::render(&report),
                TableFormat::Json => pretty(&projected(&report, Projection::BalanceChanges))?,
            };
            (common, text)
        }
        Command::FundFlow { common, format } => {
            let report = fetch(common).await?;
            let text = match format {
                FlowFormat::Mermaid => render::flow::render_mermaid(&report),
                FlowFormat::Dot => render::flow::render_dot(&report),
                FlowFormat::Json => pretty(&projected(&report, Projection::FundFlow))?,
            };
            (common, text)
        }
        Command::Report { common, compact } => {
            let report = fetch(common).await?;
            let text = if *compact {
                serde_json::to_string(&report)?
            } else {
                serde_json::to_string_pretty(&report)?
            };
            (common, text)
        }
    };

    match &common.output {
        Some(path) => {
            std::fs::write(path, output.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
            if !common.quiet {
                eprintln!("wrote {}", path.display());
            }
        }
        None => println!("{output}"),
    }
    Ok(())
}

async fn fetch(common: &Common) -> Result<TraceReport> {
    let cfg = TracerConfig {
        rpc_url: common.rpc_url.clone(),
        backend: match common.backend {
            Backend::Auto => BackendChoice::Auto,
            Backend::Rpc => BackendChoice::Rpc,
            Backend::AnvilFork => BackendChoice::AnvilFork,
        },
        deep: common.deep,
        replay: !common.no_replay,
        enrich: !common.no_enrich,
        include_gas: !common.no_gas,
        wrapped_native: common.wrapped_native,
    };
    let tracer = Tracer::connect(cfg)?;
    let report = tracer.report(common.tx_hash).await?;
    if !common.quiet {
        for w in &report.warnings {
            eprintln!("warning: {w}");
        }
    }
    Ok(report)
}

enum Projection {
    Trace,
    BalanceChanges,
    FundFlow,
}

/// Subcommand JSON keeps the envelope (schema version, tx, tokens, labels)
/// but trims the sections other commands own.
fn projected(report: &TraceReport, projection: Projection) -> TraceReport {
    let mut r = report.clone();
    match projection {
        Projection::Trace => {
            r.balance_changes = None;
            r.fund_flow = None;
        }
        Projection::BalanceChanges => {
            r.trace = None;
            r.fund_flow = None;
            r.transfers.clear();
        }
        Projection::FundFlow => {
            r.trace = None;
            r.balance_changes = None;
        }
    }
    r
}

fn pretty<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}
