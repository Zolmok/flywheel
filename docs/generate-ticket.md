# /generate-ticket

Takes a human-provided idea and turns it into a fully fleshed out GitHub
issue optimized for AI implementation. The human provides the vision; the
command provides the architectural grounding and implementation roadmap.

## Arguments

Free-form text describing the feature idea. Example:

```
/generate-ticket add email notifications when a PR is merged
```

## What it does

1. **Confirms the idea** — restates the idea and asks clarifying questions
   about who benefits, where it lives in the product, and what's out of
   scope. Waits for confirmation before proceeding.

2. **Gathers project context** — reads `.planning/PROJECT.md`,
   `.planning/STATE.md`, `CLAUDE.md`, and build config files.

3. **Analyzes the codebase** — finds insertion points, maps the call graph,
   locates reference implementations of analogous features, identifies data
   model changes, and spots potential conflicts with in-progress work.

4. **Scopes and decomposes** — determines if the idea fits in a single
   session or needs multiple tickets. If decomposed, each ticket is
   independently deployable.

5. **Creates the issue(s)** — files each issue on GitHub with:
   - Architecture context (insertion points, call graph, data flow)
   - Data model changes
   - Reference implementation from elsewhere in the codebase
   - Step-by-step implementation guide
   - Code to reuse (specific functions and paths)
   - Constraints including at least one "do not" rule and one scope boundary
   - Blast radius analysis
   - Copy-pasteable verification commands
   - Acceptance criteria checklist
   - Two labels: one category (`feature`, `enhancement`, `infrastructure`)
     and one priority (`critical`, `high`, `medium`)

## Rules

- Always confirms the idea with the user before analyzing the codebase.
- Every issue references specific files, functions, or code paths.
- Each issue is completable in a single focused session.
- The "Reference implementation" section is mandatory.
- Verification commands must be copy-pasteable with no placeholders.
- Checks for duplicate issues before creating.

## Output

A summary table of created tickets with number, title, labels, dependencies,
and one-line description.

## Installation

Save the full prompt as `~/.claude/commands/generate-ticket.md` (global) or
`.claude/commands/generate-ticket.md` (per-project). Then invoke with:

```sh
claude
> /generate-ticket add email notifications when a PR is merged
```

The full prompt source is available at:
https://github.com/Zolmok/flywheel/blob/main/docs/generate-ticket.md
