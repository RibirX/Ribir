# ribir-bot

Unified CLI for PR, changelog, and release automation for the Ribir project.

## Features

- **PR Automation**: AI-powered PR body generation (summary + changelog entries)
- **Changelog Management**: Collect, merge, and verify changelog entries using AST parsing
- **Release Automation**: Full release workflow with one command - changelog, cargo release, GitHub Release

## Installation

```bash
cargo install --path .
```

### Dependencies

- [GitHub CLI](https://cli.github.com/) (`gh`) - for GitHub operations
- [Gemini CLI](https://github.com/anthropics/gemini) (`gemini`) - for AI generation
- [cargo-release](https://crates.io/crates/cargo-release) - for version bumping and publishing

## Usage

```
ribir-bot <COMMAND> [OPTIONS]
```

### PR Commands

Update PR body with AI-generated content:

| Command | Description |
|---------|-------------|
| `pr-fill [PR_ID]` | Auto-fill placeholders in PR body |
| `pr-regen [PR_ID] [CTX]` | Regenerate all content |
| `pr-summary [PR_ID] [CTX]` | Regenerate summary only |
| `pr-entry [PR_ID] [CTX]` | Regenerate changelog entry only |

**Examples:**
```bash
ribir-bot pr-fill              # Auto-fill current PR
ribir-bot pr-regen 123         # Regenerate PR #123
ribir-bot pr-summary "be concise"  # Regenerate with context
```

### Changelog Commands

Update CHANGELOG.md:

| Command | Description |
|---------|-------------|
| `log-collect --version VER` | Collect merged PRs into changelog |
| `log-merge --version VER` | Merge pre-release versions |
| `log-verify` | Verify changelog structure |

**Examples:**
```bash
ribir-bot log-collect --version 0.5.0          # Collect PRs for 0.5.0
ribir-bot log-merge --version 0.5.0 --write    # Merge and write changes
ribir-bot log-verify                           # Verify parsing
```

### Release Commands

```
ribir-bot release <SUBCOMMAND> [OPTIONS]
```

| Subcommand | Description |
|------------|-------------|
| `next <level>` | **Full release in one command** (changelog + cargo release + GitHub Release) |
| `prepare --version VER` | Prepare RC release (archive, merge, highlights, PR) |
| `publish [PR_ID]` | Publish GitHub release |
| `promote --version VER` | Promote RC to stable |
| `highlights [CTX]` | Regenerate highlights in CHANGELOG.md |
| `verify` | Verify release state |

#### Release Levels for `next`

| Level | Example | Description |
|-------|---------|-------------|
| `alpha` | 0.5.0-alpha.54 | Development releases |
| `rc` | 0.5.0-rc.1 | Release candidates |
| `patch` | 0.5.1 | Patch releases |
| `minor` | 0.6.0 | Minor releases |
| `major` | 1.0.0 | Major releases |

**Examples:**
```bash
# Preview release (default is dry-run)
ribir-bot release next alpha

# Execute release
ribir-bot release next alpha --execute

# Promote RC to stable
ribir-bot release promote --version 0.5.0 --execute

# Verify release state
ribir-bot release verify
```

### Global Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview without applying changes |
| `--execute` | Execute changes (required for `release next` and `promote`) |
| `--write` | Write changes (for log-collect/log-merge) |
| `-h, --help` | Show help |

## Architecture

```
src/
├── main.rs       # Entry point and command dispatch
├── cli.rs        # Argument parsing and help text
├── types.rs      # Shared data types
├── utils.rs      # Utility functions
├── external.rs   # Gemini AI and GitHub CLI integration
├── changelog.rs  # Changelog AST parsing and manipulation
└── commands/
    ├── pr.rs        # PR body generation
    ├── changelog.rs # Changelog collection/merge/verify
    └── release.rs   # Release preparation and publishing
```

## How It Works

### PR Commands

1. Fetches PR data via `gh pr view`
2. Detects placeholders in PR body (marked with HTML comments)
3. Calls Gemini AI to generate summary and changelog entries
4. Updates PR body via `gh pr edit`

### Changelog Commands

1. Parses CHANGELOG.md as an AST using [comrak](https://crates.io/crates/comrak)
2. Collects merged PRs via `gh pr list`
3. Extracts changelog entries from PR bodies or titles
4. Inserts entries into the appropriate version section
5. Renders AST back to markdown

### Release Commands

#### `release next <level>` - One Command, Done

The unified release workflow:

1. **Get version**: Runs `cargo release <level> --dry-run` to determine next version
2. **Collect changelog**: Gathers entries from merged PRs into CHANGELOG.md
3. **Cargo release**: Bumps versions, commits, tags, pushes, publishes to crates.io
4. **GitHub Release**: Creates release with notes from changelog

**Dry-run mode** (default): Shows what would happen, including changelog and release notes preview.

**Execute mode** (`--execute`): Performs all operations.

#### Other Commands

- **prepare**: Archives old changelog, merges alpha entries, generates AI highlights, creates release PR
- **publish**: Creates GitHub release with release notes from changelog
- **promote**: Merges RC versions, calls cargo release, creates stable release

## Conventional Commit Types

The bot recognizes these types in PR titles and changelog entries:

| Type | Section |
|------|---------|
| `feat` | Features |
| `fix` | Fixed |
| `change` | Changed |
| `perf` | Performance |
| `docs` | Documentation |
| `breaking` | Breaking |
| `chore`, `refactor`, `internal` | Internal |

## License

Same license as the Ribir project.
