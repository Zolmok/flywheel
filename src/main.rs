use clap::Parser;
use serde::Deserialize;
use std::process::exit;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    GenerateTickets,
    SizePrioritize,
    MoveToReady,
    ImplementTicket,
    CheckReady,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::GenerateTickets => write!(f, "Generate Tickets"),
            Phase::SizePrioritize => write!(f, "Size & Prioritize"),
            Phase::MoveToReady => write!(f, "Move to Ready"),
            Phase::ImplementTicket => write!(f, "Implement Ticket"),
            Phase::CheckReady => write!(f, "Check Ready"),
        }
    }
}

#[derive(Parser)]
#[command(about = "Autonomous development loop using Claude Code")]
struct Cli {
    /// GitHub project number
    #[arg(short, long)]
    project: Option<u32>,

    /// GitHub project owner
    #[arg(short, long)]
    owner: Option<String>,

    /// Maximum full cycles (0 = indefinite)
    #[arg(short = 'c', long, default_value_t = 0)]
    max_cycles: u32,

    /// Tickets to move to Ready per cycle
    #[arg(short = 'n', long, default_value_t = 5)]
    batch_size: u32,
}

#[derive(Deserialize, Default)]
struct FileConfig {
    project: Option<u32>,
    owner: Option<String>,
}

struct Config {
    project: u32,
    owner: String,
    max_cycles: u32,
    batch_size: u32,
}

fn next_phase(current: &Phase, ready_has_items: bool) -> Phase {
    match current {
        Phase::GenerateTickets => Phase::SizePrioritize,
        Phase::SizePrioritize => Phase::MoveToReady,
        Phase::MoveToReady => Phase::ImplementTicket,
        Phase::ImplementTicket => Phase::CheckReady,
        Phase::CheckReady => {
            if ready_has_items {
                Phase::ImplementTicket
            } else {
                Phase::GenerateTickets
            }
        }
    }
}

fn merge_config(file: FileConfig, cli: &Cli) -> Result<Config, String> {
    let project = match cli.project.or(file.project) {
        Some(p) => p,
        None => return Err("Missing required field: project".to_string()),
    };

    let owner = match cli.owner.clone().or(file.owner) {
        Some(o) => o,
        None => return Err("Missing required field: owner".to_string()),
    };

    Ok(Config {
        project,
        owner,
        max_cycles: cli.max_cycles,
        batch_size: cli.batch_size,
    })
}

fn load_config(cli: &Cli) -> Config {
    let file_config = match std::fs::read_to_string(".flywheel.json") {
        Ok(contents) => match serde_json::from_str::<FileConfig>(&contents) {
            Ok(fc) => fc,
            Err(e) => {
                eprintln!("Warning: failed to parse .flywheel.json: {e}");
                FileConfig::default()
            }
        },
        Err(_) => FileConfig::default(),
    };

    match merge_config(file_config, cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Provide --project and --owner via CLI flags or .flywheel.json");
            exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let config = load_config(&cli);
    let phase = Phase::GenerateTickets;
    let second = next_phase(&phase, false);

    println!(
        "Flywheel configured: project={}, owner={}, max_cycles={}, batch_size={}",
        config.project, config.owner, config.max_cycles, config.batch_size
    );
    println!("Starting at phase: {phase}, next: {second}");
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
