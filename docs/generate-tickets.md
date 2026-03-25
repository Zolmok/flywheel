# /generate-tickets

Scans the codebase and creates up to 5 GitHub issues for the most important
bugs, security gaps, missing features, tech debt, and operational gaps.
Optionally adds each issue to a GitHub Project board.

## Arguments

- `--project <number>` — GitHub Project number to add issues to (optional)
- `--owner <owner>` — GitHub Project owner, defaults to repo owner (optional)

## What it does

1. **Gathers context** — reads `.planning/PROJECT.md`, `.planning/STATE.md`,
   `.planning/MILESTONES.md`, `CLAUDE.md`, and build config files to understand
   the project's stack, conventions, and current state. Identifies the tech
   stack, project conventions, test infrastructure, and directory structure.

2. **Analyzes the codebase** — walks the project structure looking for bugs,
   security issues, incomplete features, missing functionality, tech debt,
   and operational gaps. For each issue it records the call graph, data flow,
   and similar patterns already solved elsewhere in the codebase.

3. **Prioritizes** — ranks issues by severity: data loss / security first,
   then blocking bugs, launch requirements, high-impact features, and
   developer experience.

4. **Creates issues** — files each issue on GitHub with:
   - Specific file paths and line numbers
   - Architecture context (affected code, callers, dependencies, data flow)
   - Reference implementations from elsewhere in the codebase
   - Step-by-step implementation guide with constraints
   - Copy-pasteable verification commands
   - Acceptance criteria checklist
   - Two labels: one category (`bug`, `security`, `feature`,
     `infrastructure`, `tech-debt`) and one priority (`critical`, `high`,
     `medium`)

5. **Adds to project board** — if `--project` was specified, adds each issue
   to the board.

## Rules

- Every issue must reference specific files, functions, or code paths
- Each issue should be completable in a single focused session
- Does not create issues for things listed as "Out of Scope" in PROJECT.md
- Does not duplicate existing issues
- The "Related patterns" section is mandatory in every issue
- The "Constraints" section must include at least one "do not" rule
- Verification commands must be copy-pasteable with no placeholders

## Output

A summary table with issue number, title, labels, and one-line rationale for
each of the 5 issues created.

## Installation

Save the full prompt as `~/.claude/commands/generate-tickets.md` (global) or
`.claude/commands/generate-tickets.md` (per-project). Then invoke with:

```sh
claude
> /generate-tickets --project 3 --owner myuser
```

The full prompt source is available at:
https://github.com/Zolmok/flywheel/blob/main/docs/generate-tickets.md
