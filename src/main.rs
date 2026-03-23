use clap::Parser;
use serde::Deserialize;
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

fn spawn_and_capture(label: &str, program: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // New session so the child cannot open /dev/tty or affect our terminal
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
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
                    eprintln!("{line}");
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
                println!("{line}");
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

fn main() {
    let cli = Cli::parse();
    let config = load_config(&cli);

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
