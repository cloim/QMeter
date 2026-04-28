use clap::{Parser, ValueEnum};
use qmeter_core::output::{render_graph, render_table};
use qmeter_core::snapshot::{CollectOptions, collect_fixture_snapshot, is_fixture_mode_from_env};
use qmeter_core::types::{NormalizedSnapshot, ProviderId};
use qmeter_providers::snapshot::collect_live_snapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ViewMode {
    Table,
    Graph,
}

#[derive(Debug, Parser)]
#[command(
    name = "qmeter",
    about = "Unified usage + reset status for Claude Code and Codex.",
    disable_version_flag = true
)]
struct Cli {
    #[arg(long)]
    json: bool,

    #[arg(long)]
    refresh: bool,

    #[arg(long)]
    debug: bool,

    #[arg(long, value_enum, default_value_t = ViewMode::Table)]
    view: ViewMode,

    #[arg(long, default_value = "all")]
    providers: String,
}

fn parse_providers(raw: &str) -> Result<Vec<ProviderId>, String> {
    if raw == "all" {
        return Ok(vec![ProviderId::Claude, ProviderId::Codex]);
    }

    let mut providers = Vec::new();
    for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match part {
            "claude" => providers.push(ProviderId::Claude),
            "codex" => providers.push(ProviderId::Codex),
            _ => return Err(format!("Unknown provider: {part}")),
        }
    }

    if providers.is_empty() {
        return Err("--providers must include at least one of: claude,codex,all".to_string());
    }
    providers.sort_by_key(|id| id.as_str());
    providers.dedup();
    Ok(providers)
}

fn run(cli: Cli) -> Result<(NormalizedSnapshot, i32), String> {
    let providers = parse_providers(&cli.providers)?;
    let opts = CollectOptions {
        refresh: cli.refresh,
        debug: cli.debug,
        providers,
    };

    let snapshot = if is_fixture_mode_from_env() {
        collect_fixture_snapshot(&opts)
    } else {
        let live = collect_live_snapshot(&opts);
        if cli.debug {
            for (provider, debug) in &live.debug_messages {
                eprintln!("[debug] {}: {debug}", provider.as_str());
            }
        }
        live.snapshot
    };
    let exit_code = if !snapshot.rows.is_empty() && snapshot.errors.is_empty() {
        0
    } else if !snapshot.rows.is_empty() && !snapshot.errors.is_empty() {
        1
    } else {
        3
    };
    Ok((snapshot, exit_code))
}

fn main() {
    let cli = Cli::parse();

    let wants_json = cli.json;
    let view = cli.view;
    let (snapshot, exit_code) = match run(cli) {
        Ok(result) => result,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    if wants_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).expect("snapshot serialization should work")
        );
    } else {
        let output = match view {
            ViewMode::Table => render_table(&snapshot),
            ViewMode::Graph => render_graph(&snapshot),
        };
        println!("{output}");
    }

    std::process::exit(exit_code);
}
