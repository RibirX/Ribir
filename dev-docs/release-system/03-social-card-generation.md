# Social Card Generation

**Status:** Design Phase
**Timeline:** Post Ribir 0.5.0 stable release

---

## Overview

A standalone tool using Ribir to generate social media cards, demonstrating headless rendering capabilities while providing a useful community tool.

### Card Types

| Type | Purpose | Update Frequency |
|------|---------|------------------|
| **Project Card** | Repository homepage, documentation links | When branding changes |
| **Release Card** | Version highlights, release links | Each RC/Stable release |

---

## Goals

1. **Dogfooding** - Use Ribir to render its own social cards
2. **Community Tool** - General-purpose image generation for Rust projects
3. **Technical Showcase** - Demonstrate headless rendering capabilities

**Success Metrics:**
- All releases from 0.5.0+ use generated cards
- Zero external dependencies (no Node.js, no external APIs)
- Zero manual image editing required

---

## Solution: `ribir-render`

### CLI Tool

```bash
# Installation
cargo install ribir-render

# Generate cards
ribir-render project-card --data project.json --output project-card.png
ribir-render release-card --changelog CHANGELOG.md --version 0.5.0 --output release-card.png
```

### GitHub Action

**Repository:** `RibirX/ribir-render-action`

```yaml
- uses: RibirX/ribir-render-action@v1
  with:
    command: release-card
    changelog: CHANGELOG.md
    version: 0.5.0
    output: release-card.png
```

### Optional Online Service

Web interface and API for users who don't want to install the CLI tool.

---

## Integration with Releases

**Release Card Flow:**
```
ribir-bot log-collect collects entries
         ↓
AI generates highlights section in CHANGELOG.md
         ↓
ribir-render extracts highlights from CHANGELOG
         ↓
ribir-render generates card
         ↓
Create PR → Review → Merge → Publish
```

**Project Card Flow:**
```
Update project-card.json
         ↓
ribir-render generates asset
         ↓
Update repository social preview
```

**Note:**
- `ribir-render` reads highlights directly from `CHANGELOG.md`, eliminating the need for separate data files.
- Social cards are generated as release assets and are **not** archived in the repository.

---

## Scope

### In Scope
- CLI tool with project/release card templates
- GitHub Action wrapper
- Documentation and examples

### Out of Scope
- Animation/video export
- GUI editor
- Real-time preview

---

## Prerequisites

- Ribir stable release ready
- Project owner assigned
- Pure Rust, no external dependencies

---

## Open Questions

1. Should `ribir-render` version track Ribir versions?
2. Separate repository or under RibirX org?
3. Allow community presets via external crates?

---

## Related Documentation

- [00-release-strategy.md](00-release-strategy.md) - Overall release process
- [01-changelog-automation.md](01-changelog-automation.md) - Changelog generation with highlights and release materials

---

*Last Updated: 2026-01-02*
