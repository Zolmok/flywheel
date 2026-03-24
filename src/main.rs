use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio, exit};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Global PID of the active child session leader.
/// The stdin-watcher thread reads this to kill the entire process group.
static CHILD_PID: AtomicU32 = AtomicU32::new(0);

/// Save and restore terminal settings so we can use raw mode for Ctrl-C
/// detection while a subprocess runs.
struct RawMode {
    original: libc::termios,
}

impl RawMode {
    fn enter() -> Option<Self> {
        unsafe {
            let mut original: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut original) != 0 {
                return None;
            }
            let mut raw = original;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(0, libc::TCSANOW, &raw) != 0 {
                return None;
            }
            Some(RawMode { original })
        }
    }

    fn restore(&self) {
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.original);
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        self.restore();
    }
}

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

fn load_direnv_env() -> HashMap<String, String> {
    let output = match Command::new("direnv")
        .args(["export", "json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("direnv not available: {e}");
            return HashMap::new();
        }
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("direnv output not valid UTF-8: {e}");
            return HashMap::new();
        }
    };

    if stdout.trim().is_empty() {
        return HashMap::new();
    }

    let parsed: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse direnv JSON: {e}");
            return HashMap::new();
        }
    };

    let obj = match parsed.as_object() {
        Some(o) => o,
        None => {
            eprintln!("direnv output is not a JSON object");
            return HashMap::new();
        }
    };

    let mut env = HashMap::new();
    for (key, value) in obj {
        match value.as_str() {
            Some(v) => {
                env.insert(key.clone(), v.to_string());
            }
            None => {
                // null or non-string values are skipped (direnv uses null for unset)
            }
        }
    }
    env
}

fn resolve_claude_profile(env: &mut HashMap<String, String>) {
    let profile = match env.get("CLAUDE_PROFILE") {
        Some(p) => p.clone(),
        None => match std::env::var("CLAUDE_PROFILE") {
            Ok(p) => p,
            Err(_) => return,
        },
    };

    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };

    let profile_dir = format!("{home}/.claude/profiles/{profile}");
    let profile_path = std::path::Path::new(&profile_dir);
    if !profile_path.is_dir() {
        eprintln!("Claude profile directory not found: {profile_dir}");
        return;
    }

    let src = format!("{profile_dir}/claude.json");
    let dst = format!("{home}/.claude.json");
    match std::fs::copy(&src, &dst) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to copy {src} to {dst}: {e}");
        }
    }

    env.insert("CLAUDE_CONFIG_DIR".to_string(), profile_dir);
}

fn build_generate_tickets_prompt(config: &Config) -> String {
    format!(
        "Use the Skill tool to invoke 'generate-tickets' with arguments \
         '--project {} --owner {}'. Output the complete report.",
        config.project, config.owner
    )
}

fn build_size_prioritize_prompt(config: &Config) -> String {
    format!(
        "You are managing a GitHub Project board. Examine all items in the \"Backlog\" \
         column of project {} (owner: {}).\n\
         \n\
         For each item:\n\
         1. Read the full issue body using `gh issue view <number>`\n\
         2. Assess implementation complexity (small/medium/large)\n\
         3. Add a size label: `size:small`, `size:medium`, or `size:large`\n\
         4. Consider priority based on: severity of the problem, impact on users, \
         and implementation complexity\n\
         \n\
         Then reorder the Backlog column so the highest-priority items are at the top. \
         Use `gh project item-edit` to adjust item positions.\n\
         \n\
         Use these commands to interact with the board:\n\
         - `gh project item-list {} --owner {} --format json`\n\
         - `gh project field-list {} --owner {} --format json`\n\
         - `gh project item-edit --project-id <ID> --id <ITEM_ID> --field-id <FIELD_ID> ...`\n\
         \n\
         Output a summary table: issue number, title, size, priority rationale.",
        config.project, config.owner, config.project, config.owner, config.project, config.owner
    )
}

fn build_move_to_ready_prompt(config: &Config) -> String {
    format!(
        "You are managing a GitHub Project board. Move the top {} items \
         from the \"Backlog\" column to the \"Ready\" column in project {} \
         (owner: {}).\n\
         \n\
         Steps:\n\
         1. List items: `gh project item-list {} --owner {} --format json`\n\
         2. Get field metadata: `gh project field-list {} --owner {} --format json`\n\
         3. For each of the top {} Backlog items, change status to \"Ready\":\n\
            `gh project item-edit --project-id <ID> --id <ITEM_ID> \
         --field-id <STATUS_FIELD_ID> --single-select-option-id <READY_OPTION_ID>`\n\
         \n\
         If there are fewer than {} items in Backlog, move all of them.\n\
         \n\
         Output a summary of which items were moved.",
        config.batch_size,
        config.project,
        config.owner,
        config.project,
        config.owner,
        config.project,
        config.owner,
        config.batch_size,
        config.batch_size
    )
}

fn build_implement_ticket_prompt(config: &Config) -> String {
    format!(
        "Use the Skill tool to invoke 'implement-ticket' with arguments \
         '--project {} --owner {}'. Output the complete report.",
        config.project, config.owner
    )
}

struct TicketInfo {
    number: u64,
    title: String,
}

#[allow(clippy::question_mark)]
fn parse_top_ready_ticket(json: &str) -> Option<TicketInfo> {
    let value = match serde_json::from_str::<serde_json::Value>(json) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let items_value = match value.get("items") {
        Some(v) => v,
        None => return None,
    };
    let items = match items_value.as_array() {
        Some(arr) => arr,
        None => return None,
    };
    for item in items {
        let status = match item.get("status") {
            Some(s) => match s.as_str() {
                Some(s) => s,
                None => continue,
            },
            None => continue,
        };
        if status != "Ready" {
            continue;
        }
        let number = match item.get("content") {
            Some(content) => match content.get("number") {
                Some(n) => match n.as_u64() {
                    Some(n) => n,
                    None => continue,
                },
                None => continue,
            },
            None => continue,
        };
        let title = match item.get("title") {
            Some(t) => match t.as_str() {
                Some(t) => t.to_string(),
                None => continue,
            },
            None => continue,
        };
        return Some(TicketInfo { number, title });
    }
    None
}

fn count_backlog_items(json: &str) -> usize {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => match value.get("items") {
            Some(items) => match items.as_array() {
                Some(arr) => arr
                    .iter()
                    .filter(|item| match item.get("status") {
                        Some(status) => match status.as_str() {
                            Some(s) => s == "Backlog",
                            None => false,
                        },
                        None => false,
                    })
                    .count(),
                None => 0,
            },
            None => 0,
        },
        Err(_) => 0,
    }
}

fn count_ready_items(json: &str) -> usize {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => match value.get("items") {
            Some(items) => match items.as_array() {
                Some(arr) => arr
                    .iter()
                    .filter(|item| match item.get("status") {
                        Some(status) => match status.as_str() {
                            Some(s) => s == "Ready",
                            None => false,
                        },
                        None => false,
                    })
                    .count(),
                None => 0,
            },
            None => 0,
        },
        Err(_) => 0,
    }
}

fn check_backlog_count(config: &Config, extra_env: &HashMap<String, String>) -> usize {
    let project_str = config.project.to_string();
    let output = spawn_and_capture(
        "check-backlog",
        "gh",
        &[
            "project",
            "item-list",
            &project_str,
            "--owner",
            &config.owner,
            "--format",
            "json",
        ],
        extra_env,
        true,
    );
    match output {
        Some(json) => count_backlog_items(&json),
        None => 0,
    }
}

fn check_ready_column(config: &Config, extra_env: &HashMap<String, String>) -> (bool, usize) {
    let project_str = config.project.to_string();
    let output = spawn_and_capture(
        "check-ready",
        "gh",
        &[
            "project",
            "item-list",
            &project_str,
            "--owner",
            &config.owner,
            "--format",
            "json",
        ],
        extra_env,
        true,
    );
    match output {
        Some(json) => {
            let count = count_ready_items(&json);
            (count > 0, count)
        }
        None => (false, 0),
    }
}

fn get_top_ready_ticket(
    config: &Config,
    extra_env: &HashMap<String, String>,
) -> Option<TicketInfo> {
    let project_str = config.project.to_string();
    let output = spawn_and_capture(
        "get-top-ready-ticket",
        "gh",
        &[
            "project",
            "item-list",
            &project_str,
            "--owner",
            &config.owner,
            "--format",
            "json",
        ],
        extra_env,
        true,
    );
    match output {
        Some(json) => parse_top_ready_ticket(&json),
        None => None,
    }
}

fn run_phase(phase: &Phase, config: &Config, extra_env: &HashMap<String, String>) -> Option<Phase> {
    match phase {
        Phase::CheckReady => {
            let (has_items, count) = check_ready_column(config, extra_env);
            if count == 0 {
                println!("Ready column: empty");
            } else {
                println!("Ready column: {count} item(s)");
            }
            Some(next_phase(phase, has_items))
        }
        Phase::GenerateTickets => {
            let backlog_count = check_backlog_count(config, extra_env);
            if backlog_count >= 5 {
                println!("Backlog has {backlog_count} items, skipping ticket generation");
                return Some(Phase::SizePrioritize);
            }
            let prompt = build_generate_tickets_prompt(config);
            let result = spawn_and_capture(
                &format!("{phase}"),
                "claude",
                &["-p", &prompt, "--dangerously-skip-permissions"],
                extra_env,
                false,
            );
            result.map(|_| next_phase(phase, false))
        }
        _ => {
            let prompt = match phase {
                Phase::SizePrioritize => build_size_prioritize_prompt(config),
                Phase::MoveToReady => build_move_to_ready_prompt(config),
                Phase::ImplementTicket => build_implement_ticket_prompt(config),
                Phase::CheckReady | Phase::GenerateTickets => unreachable!(),
            };
            let result = spawn_and_capture(
                &format!("{phase}"),
                "claude",
                &["-p", &prompt, "--dangerously-skip-permissions"],
                extra_env,
                false,
            );
            result.map(|_| next_phase(phase, false))
        }
    }
}

fn spawn_and_capture(
    label: &str,
    program: &str,
    args: &[&str],
    extra_env: &HashMap<String, String>,
    quiet: bool,
) -> Option<String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .envs(extra_env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // New process group so we can kill the child tree on Ctrl-C, but stay
    // in the same session so the child can still reach /dev/tty (needed for
    // OAuth token refresh in claude).
    unsafe {
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to spawn {program}: {e}");
            return None;
        }
    };

    // Store child PID so the stdin-watcher can kill the process group
    CHILD_PID.store(child.id(), Ordering::Relaxed);

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            eprintln!("{label}: no stdout pipe");
            return None;
        }
    };
    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            eprintln!("{label}: no stderr pipe");
            return None;
        }
    };

    let output = Arc::new(Mutex::new(String::new()));

    // Spawn a thread to stream stderr to terminal AND capture it
    let output_clone = output.clone();
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if !quiet {
                        eprintln!("{line}");
                    }
                    match output_clone.lock() {
                        Ok(mut out) => {
                            out.push_str(&line);
                            out.push('\n');
                        }
                        Err(e) => {
                            eprintln!("stderr lock poisoned: {e}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("stderr read error: {e}");
                    break;
                }
            }
        }
    });

    // Stream stdout to terminal AND capture it
    let output_clone2 = output.clone();
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if !quiet {
                    println!("{line}");
                }
                match output_clone2.lock() {
                    Ok(mut out) => {
                        out.push_str(&line);
                        out.push('\n');
                    }
                    Err(e) => {
                        eprintln!("{label}: stdout lock poisoned: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("{label}: stdout read error: {e}");
                break;
            }
        }
    }

    drop(output_clone2);
    let _ = stderr_handle.join();

    let status = child.wait();
    CHILD_PID.store(0, Ordering::Relaxed);

    let output = match Arc::try_unwrap(output) {
        Ok(mutex) => match mutex.into_inner() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{label}: mutex poisoned: {e}");
                e.into_inner()
            }
        },
        Err(arc) => {
            eprintln!("{label}: Arc still has multiple owners, cloning");
            match arc.lock() {
                Ok(s) => s.clone(),
                Err(e) => {
                    eprintln!("{label}: fallback lock poisoned: {e}");
                    e.into_inner().clone()
                }
            }
        }
    };

    match status {
        Ok(s) => {
            if !s.success() {
                eprintln!("{label}: {program} exited with {s}");
            }
            Some(output)
        }
        Err(e) => {
            eprintln!("{label}: failed to wait on {program}: {e}");
            None
        }
    }
}

fn print_phase_banner(phase: &Phase, cycle: u32, ticket: Option<&TicketInfo>) {
    println!("=========================================");
    println!("  Flywheel \u{2014} cycle {cycle}");
    match ticket {
        Some(info) => println!("  Phase: {phase} \u{2014} #{}: {}", info.number, info.title),
        None => println!("  Phase: {phase}"),
    }
    println!("=========================================");
}

fn main() {
    let cli = Cli::parse();
    let config = load_config(&cli);
    let mut direnv_env = load_direnv_env();
    resolve_claude_profile(&mut direnv_env);

    let _raw_mode = RawMode::enter();

    // Spawn a stdin-watcher thread that reads for Ctrl-C bytes.
    // When detected, kill the child process group and exit.
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut buf = [0u8; 1];
        loop {
            match stdin.lock().read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    if buf[0] == 0x03 {
                        let pid = CHILD_PID.load(Ordering::Relaxed);
                        if pid != 0 {
                            unsafe {
                                libc::kill(-(pid as i32), libc::SIGKILL);
                            }
                        }
                        eprintln!("\n=== Interrupted ===");
                        // Restore terminal before exiting
                        unsafe {
                            let mut original: libc::termios = std::mem::zeroed();
                            libc::tcgetattr(0, &mut original);
                            original.c_lflag |= libc::ICANON | libc::ECHO | libc::ISIG;
                            libc::tcsetattr(0, libc::TCSANOW, &original);
                        }
                        std::process::exit(1);
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut phase = Phase::GenerateTickets;
    let mut cycle: u32 = 1;

    loop {
        let ticket_info = match phase {
            Phase::ImplementTicket => get_top_ready_ticket(&config, &direnv_env),
            _ => None,
        };
        print_phase_banner(&phase, cycle, ticket_info.as_ref());

        match run_phase(&phase, &config, &direnv_env) {
            None => {
                eprintln!("=== Phase \"{}\" failed, stopping ===", phase);
                break;
            }
            Some(next) => {
                println!("--- {} complete, moving to {} ---", phase, next);

                if phase == Phase::CheckReady && next == Phase::GenerateTickets {
                    println!(
                        "=== Cycle {} complete, starting cycle {} ===",
                        cycle,
                        cycle + 1
                    );
                    cycle += 1;
                    if config.max_cycles > 0 && cycle > config.max_cycles {
                        println!("=== Reached max cycles ({}) ===", config.max_cycles);
                        break;
                    }
                }

                phase = next;
            }
        }
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
