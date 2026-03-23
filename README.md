# Flywheel

An autonomous development loop that uses Claude Code to generate tickets,
size and prioritize them, and implement them from a GitHub Project board.
Flywheel runs continuously, cycling through phases until all work is done or
a cycle limit is reached.

## Prerequisites

- **Rust** (edition 2024)
- **`claude` CLI** ([Claude Code](https://claude.ai/code))
- **`gh` CLI** ([GitHub CLI](https://cli.github.com/), authenticated)
- A **GitHub Project board** with at least Backlog and Ready columns

## Installation

```sh
cargo build --release
```

The binary is written to `target/release/flywheel`.

## Configuration

Flywheel reads configuration from CLI flags and an optional `.flywheel.json`
file in the current directory. CLI flags override file values.

### `.flywheel.json`

```json
{
  "project": 1,
  "owner": "your-github-username"
}
```

### CLI flags

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--project` | `-p` | — | GitHub project number (required) |
| `--owner` | `-o` | — | GitHub project owner (required) |
| `--max-cycles` | `-c` | `0` | Maximum full cycles (0 = indefinite) |
| `--batch-size` | `-n` | `5` | Tickets to move to Ready per cycle |

Both `project` and `owner` must be provided via CLI flags or
`.flywheel.json`. If either is missing, Flywheel exits with an error.

### Environment: direnv

Flywheel automatically loads environment variables from `.envrc` via
[direnv](https://direnv.net/) when present. This enables per-project
configuration such as different API keys or tool settings.

Example `.envrc`:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
```

If direnv is not installed or no `.envrc` exists, Flywheel behaves normally
using the inherited environment.

## Usage

```sh
# Run with CLI flags
flywheel --project 1 --owner myuser

# Limit to 3 cycles with a larger batch
flywheel --max-cycles 3 --batch-size 10

# Use .flywheel.json for project/owner, override batch size
flywheel --batch-size 8
```

## How It Works

Flywheel drives a 5-phase cycle that repeats until the cycle limit is
reached or an error occurs:

1. **Generate Tickets** — invokes the `/generate-tickets` Claude Code slash
   command to scan the codebase and create GitHub issues for improvements,
   bugs, and missing features.

2. **Size & Prioritize** — examines all Backlog items, assesses complexity,
   adds size labels, and reorders by priority.

3. **Move to Ready** — moves the top N items (controlled by `--batch-size`)
   from Backlog to the Ready column.

4. **Implement Ticket** — invokes the `/implement-ticket` Claude Code slash
   command to pick the top Ready ticket, implement it, write tests, and
   open a pull request.

5. **Check Ready** — queries the project board for remaining Ready items.
   If any exist, loops back to phase 4. If none remain, starts a new cycle
   from phase 1.

## Claude Code Slash Commands

Flywheel delegates work to two Claude Code slash commands. These must be
installed in `~/.claude/commands/` (global) or `.claude/commands/`
(per-project) for Flywheel to function. Full documentation for each:

- **[`/generate-tickets`](docs/generate-tickets.md)** — scans the codebase,
  identifies the 5 most important issues, and creates them on GitHub with
  architecture context, implementation guides, and acceptance criteria.

- **[`/implement-ticket`](docs/implement-ticket.md)** — picks the top Ready
  ticket from the project board, implements it, writes tests, opens a PR,
  and moves the ticket to "In Review".

## Development

See [CLAUDE.md](CLAUDE.md) for code conventions and architecture rules.

```sh
cargo build          # build
cargo test           # run tests
cargo fmt -- --check # check formatting
cargo clippy         # lint
```
