# Developer Documentation

This directory contains internal documentation for Ribir project maintainers and contributors.

---

## 📚 Documentation Topics

| Topic | Description |
|-------|-------------|
| [Release System](release-system/) | Complete release workflow: changelog automation, versioning strategy, social cards, and CI/CD workflows |

---

## 🔧 Tools Quick Reference

All tools are self-documented. Run with `--help` for usage:

```bash
# Ribir Bot - Unified CLI for PR, changelog, and release automation
cargo run -p ribir-bot -- --help

# PR subcommand - AI-powered PR summary and changelog generation
cargo run -p ribir-bot -- pr --help

# Changelog subcommand - Collect and manage changelog entries
cargo run -p ribir-bot -- changelog --help

# Release subcommand - RC/Stable release automation
cargo run -p ribir-bot -- release --help
```

---

## 📖 Related Root-Level Docs

| Document | Description |
|----------|-------------|
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Contribution guidelines |
| [RELEASE.md](../RELEASE.md) | Release process and branch management |
| [ROADMAP.md](../ROADMAP.md) | Project roadmap |
