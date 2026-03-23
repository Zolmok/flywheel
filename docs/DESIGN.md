# Flywheel — Design Document

## Overview

Flywheel is an autonomous development loop CLI that continuously generates, prioritizes, and implements work items using Claude Code. It orchestrates a full development cycle: analyze a codebase for improvements, create GitHub issues, prioritize them on a kanban board, and implement them one by one — then repeat.

### Motivation

The existing `review-loop` CLI runs code review skills iteratively until a codebase is clean. Flywheel extends this concept from review/fix cycles to full feature development cycles. Where review-loop answers "is this code good?", flywheel answers "what should we build next?" and then builds it.

### Relationship to review-loop

Flywheel mirrors review-loop's architecture exactly — single-binary Rust CLI, same code style, same subprocess spawning patterns, same terminal handling. The key structural difference is replacing review-loop's simple iteration counter with a state machine that models the multi-phase development workflow.

---

## Workflow — State Machine

```
┌──────────────────┐
│ GenerateTickets   │──→ Creates 5 GitHub issues in Backlog
└────────┬─────────┘
         ▼
┌──────────────────┐
│ SizePrioritize    │──→ Adds size labels, reorders Backlog by priority
└────────┬─────────┘
         ▼
┌──────────────────┐
│ MoveToReady       │──→ Moves top N tickets from Backlog → Ready
└────────┬─────────┘
         ▼
┌──────────────────┐
│ ImplementTicket   │──→ Implements first Ready ticket, creates PR
└────────┬─────────┘
         ▼
┌──────────────────┐     has items
│ CheckReady        │────────────→ ImplementTicket
└────────┬─────────┘
         │ empty
         ▼
   GenerateTickets (new cycle)
```

### Phase Transitions

| Current Phase     | Next Phase        | Condition                    |
|-------------------|-------------------|------------------------------|
| GenerateTickets   | SizePrioritize    | Always                       |
| SizePrioritize    | MoveToReady       | Always                       |
| MoveToReady       | ImplementTicket   | Always                       |
| ImplementTicket   | CheckReady        | Always                       |
| CheckReady        | ImplementTicket   | Ready column has items       |
| CheckReady        | GenerateTickets   | Ready column is empty        |

Any phase returning an error stops the loop.

---

## Phase Details

### Phase 1: GenerateTickets

**Method**: Spawn `claude` invoking the `generate-tickets` skill.

**Prompt**:
```
Use the Skill tool to invoke 'generate-tickets' with arguments
'--project <PROJECT> --owner <OWNER>'. Output the complete report.
```

**What it does**: The `generate-tickets` skill (defined at `~/.claude/commands/generate-tickets.md`) analyzes the codebase for bugs, security issues, incomplete features, tech debt, and operational gaps. It creates 5 GitHub issues with detailed architecture context, implementation guides, and verification steps. Issues are added to the project board's Backlog column.

**Completion**: Phase succeeds when Claude exits with status 0 and output contains the summary table of created issues.

### Phase 2: SizePrioritize

**Method**: Spawn `claude` with a custom prompt (no skill invocation).

**Prompt**:
```
You are managing a GitHub Project board. Examine all items in the "Backlog"
column of project <PROJECT> (owner: <OWNER>).

For each item:
1. Read the full issue body using `gh issue view <number>`
2. Assess implementation complexity (small/medium/large)
3. Add a size label: `size:small`, `size:medium`, or `size:large`
4. Consider priority based on: severity of the problem, impact on users,
   and implementation complexity

Then reorder the Backlog column so the highest-priority items are at the top.
Use `gh project item-edit` to adjust item positions.

Use these commands to interact with the board:
- `gh project item-list <PROJECT> --owner <OWNER> --format json`
- `gh project field-list <PROJECT> --owner <OWNER> --format json`
- `gh project item-edit --project-id <ID> --id <ITEM_ID> --field-id <FIELD_ID> ...`

Output a summary table: issue number, title, size, priority rationale.
```

**Completion**: Phase succeeds when Claude exits with status 0.

### Phase 3: MoveToReady

**Method**: Spawn `claude` with a custom prompt (no skill invocation).

**Prompt**:
```
You are managing a GitHub Project board. Move the top <BATCH_SIZE> items
from the "Backlog" column to the "Ready" column in project <PROJECT>
(owner: <OWNER>).

Steps:
1. List items: `gh project item-list <PROJECT> --owner <OWNER> --format json`
2. Get field metadata: `gh project field-list <PROJECT> --owner <OWNER> --format json`
3. For each of the top <BATCH_SIZE> Backlog items, change status to "Ready":
   `gh project item-edit --project-id <ID> --id <ITEM_ID> --field-id <STATUS_FIELD_ID> --single-select-option-id <READY_OPTION_ID>`

If there are fewer than <BATCH_SIZE> items in Backlog, move all of them.

Output a summary of which items were moved.
```

**Completion**: Phase succeeds when Claude exits with status 0.

### Phase 4: ImplementTicket

**Method**: Spawn `claude` invoking the `implement-ticket` skill.

**Prompt**:
```
Use the Skill tool to invoke 'implement-ticket' with arguments
'--project <PROJECT> --owner <OWNER>'. Output the complete report.
```

**What it does**: The `implement-ticket` skill (defined at `~/.claude/commands/implement-ticket.md`) selects the first Ready item, moves it to In Progress, implements the change, writes tests, creates a branch via `gh issue develop`, commits, creates a PR, monitors CI, and moves the ticket to In Review.

**Completion**: Phase succeeds when Claude exits with status 0.

**Error handling**: If the skill reports that all tickets are blocked or too large, the phase returns an error and the loop stops. This requires parsing the output for failure indicators.

### Phase 5: CheckReady

**Method**: Run `gh` directly (no Claude invocation needed).

**Command**:
```
gh project item-list <PROJECT> --owner <OWNER> --format json
```

**Logic**: Parse the JSON output, filter for items with status "Ready". If any exist, transition to ImplementTicket. If none, transition to GenerateTickets (new cycle).

**Why not use Claude?**: This is a deterministic check — no reasoning needed. Running `gh` directly is faster, cheaper, and more reliable than spawning a Claude session for a simple query.

---

## Architecture

### Single-Binary CLI

Following review-loop's pattern, all code lives in `src/main.rs` with tests in `src/main_tests.rs`. No library crate, no module tree.

### Project Structure

```
flywheel/
├── Cargo.toml
├── Cargo.lock
├── CLAUDE.md
├── README.md
├── .gitignore
├── .dev.json
├── docs/
│   └── DESIGN.md          # This file
└── src/
    ├── main.rs             # All application logic (~500-600 lines)
    └── main_tests.rs       # Unit tests (~100-130 lines)
```

No `skills/` directory needed — the `generate-tickets` and `implement-ticket` skills already exist globally at `~/.claude/commands/`.

### Dependencies

```toml
[package]
name = "flywheel"
version = "0.1.0"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
libc = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

| Dependency   | Purpose                                          |
|--------------|--------------------------------------------------|
| `clap`       | CLI argument parsing (derive macros)             |
| `libc`       | Unix terminal control (termios, signals, setsid) |
| `serde`      | Deserialize `.flywheel.json` config              |
| `serde_json` | Parse `.flywheel.json` and `gh` JSON output      |

Note: `regex` is NOT needed (unlike review-loop). Flywheel doesn't parse structured review output — it either invokes skills that handle their own completion, or checks a column via `gh` JSON output.

---

## Configuration

### CLI Arguments

```rust
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
```

### Config File (`.flywheel.json`)

Optional file in the current working directory:

```json
{
  "project": 1,
  "owner": "zolmok"
}
```

### Resolution Order

1. Read `.flywheel.json` from current directory (if it exists)
2. CLI flags override any values from the config file
3. `project` and `owner` must be resolved by the time the app starts — if missing from both sources, exit with an error message

---

## Code Reuse from review-loop

The following sections of `review-loop/src/main.rs` can be copied directly with minimal or no modification:

### RawMode struct (lines 15-49)

Manages Unix terminal settings via `libc::termios`. Disables canonical mode, echo, and signals for raw byte reads. Restores original settings on drop. **Copy verbatim.**

### CHILD_PID static (line 11)

```rust
static CHILD_PID: AtomicU32 = AtomicU32::new(0);
```

Global storage for the current child process PID, used by the interrupt handler. **Copy verbatim.**

### spawn_and_capture() function (lines 82-213)

Spawns a subprocess (`claude` or `gh`), streams stdout/stderr to the terminal while capturing to strings. Uses `setsid()` for process isolation, `Arc<Mutex<String>>` for thread-safe capture, and returns the captured output. **Copy verbatim** — the `label` parameter already makes it generic for any phase.

### Stdin-watcher / Ctrl-C handler (lines 424-453)

Background thread that reads raw stdin bytes, detects Ctrl-C (0x03), and kills the child process group via `libc::kill(-(pid), SIGKILL)`. Restores terminal settings before exiting. **Copy verbatim.**

### What's NOT reused

- `parse_issues()` and the three parser functions — not needed, flywheel doesn't parse review output
- `build_fix_prompt()` — replaced by flywheel's own prompt builders
- `Task` enum — replaced by `Phase` enum
- The review→fix iteration loop — replaced by the state machine

---

## New Code to Write

### Phase enum (~20 lines)

```rust
#[derive(Debug, Clone, PartialEq)]
enum Phase {
    GenerateTickets,
    SizePrioritize,
    MoveToReady,
    ImplementTicket,
    CheckReady,
}
```

With `Display` impl for human-readable banner output.

### Config struct and loader (~50 lines)

```rust
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
```

`load_config()` reads `.flywheel.json` (if present), merges with CLI args, validates required fields.

### Prompt builders (~80 lines)

Five pure functions, one per phase:

```rust
fn build_generate_tickets_prompt(config: &Config) -> String
fn build_size_prioritize_prompt(config: &Config) -> String
fn build_move_to_ready_prompt(config: &Config) -> String
fn build_implement_ticket_prompt(config: &Config) -> String
```

No prompt builder for CheckReady — it runs `gh` directly.

### Phase runner (~60 lines)

```rust
fn run_phase(phase: &Phase, config: &Config) -> Option<Phase>
```

Dispatches to per-phase logic. For GenerateTickets through ImplementTicket, constructs the prompt and calls `spawn_and_capture("claude", ...)`. For CheckReady, calls `check_ready_column()`.

### Ready column checker (~30 lines)

```rust
fn check_ready_column(config: &Config) -> bool
```

Runs `gh project item-list <project> --owner <owner> --format json`, parses the JSON output with `serde_json`, filters for items with status "Ready", returns `true` if any exist.

### Main loop (~50 lines)

```rust
fn main() {
    let cli = Cli::parse();
    let config = load_config(&cli);

    // Set up raw mode and stdin-watcher (from review-loop)

    let mut phase = Phase::GenerateTickets;
    let mut cycle: u32 = 1;

    loop {
        print_phase_banner(&phase, cycle);

        match run_phase(&phase, &config) {
            None => {
                eprintln!("Phase {} failed, stopping.", phase);
                break;
            }
            Some(next_phase) => {
                if phase == Phase::CheckReady && next_phase == Phase::GenerateTickets {
                    cycle += 1;
                    if config.max_cycles > 0 && cycle > config.max_cycles {
                        println!("=== Reached max cycles ({}) ===", config.max_cycles);
                        break;
                    }
                }
                phase = next_phase;
            }
        }
    }
}
```

---

## Testing Strategy

Tests go in `src/main_tests.rs` following review-loop's pattern.

### Unit tests for prompt builders

- Each prompt includes the correct project number and owner
- `build_move_to_ready_prompt` includes the batch size
- Prompts contain the expected skill names or `gh` commands

### Unit tests for config loading

- CLI flags override file config values
- Missing required fields produce an error
- File config with all fields works
- Missing `.flywheel.json` falls back to CLI-only

### Unit tests for phase transitions

- CheckReady with non-empty Ready list → ImplementTicket
- CheckReady with empty Ready list → GenerateTickets
- All other phases transition to their fixed next phase

### Unit tests for ready column parsing

- Empty JSON response → returns false
- JSON with items all in "Backlog" → returns false
- JSON with items in "Ready" → returns true

### Integration testing (manual)

Run `flywheel --project <N> --owner <owner>` in a test repo and verify each phase invokes correctly. Since the app spawns external processes (claude, gh), automated integration tests are impractical.

---

## Output Format

Each phase prints a banner before execution:

```
=========================================
  Flywheel — cycle 1
  Phase: Generate Tickets
=========================================
```

Claude subprocess output streams directly to the terminal (same as review-loop).

Between phases, a brief transition line:

```
--- Generate Tickets complete, moving to Size & Prioritize ---
```

Cycle completion:

```
=== Cycle 1 complete, starting cycle 2 ===
```

Termination:

```
=== Reached max cycles (3) ===
```

or

```
=== Phase "Implement Ticket" failed, stopping ===
```

---

## Potential Challenges

### Prompt engineering for board management

The SizePrioritize and MoveToReady phases rely on Claude correctly using `gh project` CLI commands. These prompts may need iteration — the `gh project` API has specific syntax for field IDs, option IDs, and item edits that Claude needs to get right.

### GitHub Projects API complexity

Getting field IDs and option IDs requires multiple `gh` calls (`field-list`, then parsing for the Status field and its options). The prompts must guide Claude through this multi-step process.

### Long-running stability

Flywheel can run for hours or days. Terminal state, child process cleanup, and signal handling must be robust. The existing RawMode/SIGKILL pattern from review-loop handles this well, but worth monitoring for edge cases (e.g., orphaned Claude processes).

### Rate limits

Spawning many Claude sessions back-to-back could hit API rate limits. Consider adding an optional `--delay <seconds>` CLI flag for a pause between phases if this becomes an issue. Not needed for v1.

### Implement-ticket failures

If a ticket is too complex or blocked, `implement-ticket` may report failure. Flywheel should detect this (parse output for "cannot fully implement" or non-zero exit code) and either skip to the next Ready ticket or stop the loop. For v1, non-zero exit code stops the loop.
