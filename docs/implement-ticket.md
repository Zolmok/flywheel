# /implement-ticket

Selects the highest-priority "Ready" ticket from a GitHub Project board,
implements it fully, writes tests, commits, opens a pull request, and
moves the ticket to "In Review".

## Arguments

- `--project <number>` — GitHub Project number (required)
- `--owner <owner>` — GitHub Project owner, defaults to repo owner
- `--ticket <number>` — (optional) specific issue number to implement;
  skips board selection when provided

Arguments may also be provided in natural-language form:
`do ticket <number> on project <number> under <owner>`.

## What it does

1. **Selects a ticket** — if `--ticket` is provided, uses that specific
   issue directly. Otherwise, lists items on the project board, filters
   for "Ready" status, and picks the first one (pre-sorted by priority).
   Moves it to "In Progress" before starting work.

2. **Understands the ticket** — reads the full issue body, referenced files,
   and `CLAUDE.md` for project conventions.

3. **Plans** — lists files to create or modify, maps acceptance criteria to
   code changes, identifies needed tests and migrations.

4. **Implements** — writes the code following all conventions in `CLAUDE.md`.
   Creates new migrations if needed (never modifies existing ones). For Rust
   projects, runs `cargo clippy -- -D warnings` and fixes all errors and
   warnings — the PR must leave clippy completely clean.

5. **Writes unit tests** — uses the `rust-test-guardian` agent for Rust or
   `frontend-test-guardian` for JS/TS. Ensures all tests pass.

6. **Writes e2e tests** — if Playwright infrastructure exists, creates e2e
   tests covering every acceptance criterion.

7. **Branches and commits** — pulls latest main, creates a feature branch
   with `gh issue develop` (linking the issue in the Development sidebar),
   stages relevant files, and commits with the format
   `#<issue>: <summary>`.

8. **Opens a PR** — pushes the branch, creates a pull request with
   `Resolves #<issue>` in the body. The `gh issue develop` link ensures
   the issue auto-closes when the PR is merged.

9. **Moves to "In Review"** — updates the ticket status on the project
   board. Does **not** merge or move to "Done".

10. **Reports** — outputs a summary with issue link, PR link, board status,
    changes made, and tests added.

## Rules

- Every acceptance criterion must be implemented and tested.
- Never modifies existing migration files.
- Never merges the PR — it stays open for human review.
- Never commits failing code.
- All fix commits go through the open PR, never directly to main.
- Always creates the branch with `gh issue develop` so the issue is linked.

## Output

A structured report with the issue, PR URL, what was changed, tests added,
and board status.

## Installation

Save the full prompt as `~/.claude/commands/implement-ticket.md` (global) or
`.claude/commands/implement-ticket.md` (per-project). Then invoke with:

```sh
claude
> /implement-ticket --project 3 --owner myuser
```

The full prompt source is available at:
https://github.com/Zolmok/flywheel/blob/main/docs/implement-ticket.md
