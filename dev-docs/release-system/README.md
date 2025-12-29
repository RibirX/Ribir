# Discoverable Changelog & Release System

**Date:** 2025-12-30
**Status:** Draft

## Overview

This directory contains the design documentation for Ribir's release and changelog system. The system automates the entire flow from PR to published release with high-quality changelogs and social cards.

## Problem Statement

- Manual effort and inconsistency in maintaining high-quality changelogs
- Lack of attractive release social cards for sharing on social media

## Solution

An automated release system with:
- **Changelog automation**: AI-powered PR bot and changelog collection
- **Streamlined flow**: Alpha (weekly) → RC (1-2 weeks) → Stable
- **Release materials**: CHANGELOG with embedded highlights + social cards
- **Human-in-the-loop**: Automated generation with manual review via PR

## Documentation Structure

### Core Handbook (User-Facing)

**0. [Release Strategy](00-release-strategy.md)** ⭐ Start here
   - Release flow overview (Alpha → RC → Stable → Patch)
   - When to use each release type (decision matrix)
   - Design principles and rationale

**1. [Changelog Automation](01-changelog-automation.md)**
   - PR Bot: Auto-generate summaries and changelog entries
   - Changelog Bot: Collect and merge entries
   - Release materials: Highlights in CHANGELOG and social cards
   - Main commands and usage

**2. [Complete Release Flow](02-complete-flow.md)**
   - Step-by-step operations for each release type
   - How to trigger workflows
   - Verification checklists
   - Troubleshooting common issues

**3. [Social Card Generation](03-social-card-generation.md)**
   - Standalone ribir-render tool design
   - Visual asset generation for releases

---

## Automation Tools

### Ribir Bot (`tools/ribir-bot`)

Unified CLI tool for PR, changelog, and release automation. Uses a flat command structure for simplicity:

```
PR Commands           Changelog Commands       Release Commands
────────────          ──────────────────       ────────────────
pr-fill               log-collect              release prepare
pr-regen              log-merge                release publish
pr-summary            log-verify               release promote
pr-entry                                       release verify
```

**PR Commands** (`pr-*`)
- `pr-fill` - Auto-fill placeholders in PR body
- `pr-regen` - Regenerate all AI content (summary + changelog)
- `pr-summary` - Regenerate summary only
- `pr-entry` - Regenerate changelog entry only

**Changelog Commands** (`log-*`)
- `log-collect` - Collect entries from merged PRs into CHANGELOG.md
- `log-merge` - Merge pre-release version entries
- `log-verify` - Verify changelog structure and parsing

**Release Commands** (`release *`)
- `release prepare` - Prepare RC release (archive, merge, AI highlights, PR)
- `release publish` - Publish GitHub release
- `release promote` - Promote RC to stable
- `release verify` - Verify release state

### GitHub Actions Workflows

**Alpha Release** (automated weekly)
- Runs every Tuesday at 14:00 UTC
- Auto-increments version (e.g., 0.5.0-alpha.23 → alpha.24)
- Collects changelog entries and creates GitHub Release
- Can be triggered manually for on-demand releases

**RC Preparation** (manual)
- Merges all alpha changelogs
- AI generates highlights section in CHANGELOG.md
- Generates social card preview (future)
- Creates PR for human review

**RC Publishing** (automatic on PR merge)
- Triggers when RC preparation PR is merged
- Creates release branch (`release-0.5.x`)
- Generates final social card (future)
- Publishes GitHub Release with materials

**Stable Release** (manual)
- Promotes RC to stable after testing period (1-2 weeks)
- Reuses RC materials (highlights in CHANGELOG, social card)
- Removes pre-release flag
- Finalized version published to GitHub and crates.io

**Patch Release** (manual)
- Quick bug fix releases on release branch
- Collects only bug fix changelog entries
- No social cards or highlights (not needed for patches)
- Immediate publication

## Quick Start

**New to the release system?**

1. **[Release Strategy](00-release-strategy.md)** - Understand the overall flow
2. **[Changelog Automation](01-changelog-automation.md)** - Learn how changelog works and release materials
3. **[Complete Release Flow](02-complete-flow.md)** - Follow operations for your release type
4. **[Social Card Generation](03-social-card-generation.md)** - Understand ribir-render tooling

**Need technical details?**
- See `implementation/` directory for workflows, tools, and architecture

## Related Documentation

- [Release Strategy](00-release-strategy.md) - Release flow and versioning
- [Changelog Automation](01-changelog-automation.md) - PR bot and changelog automation
- [RELEASE.md](../../../RELEASE.md) - Release process documentation
- [PR Template](../../../.github/pull_request_template.md) - PR workflow

## Status

- [x] Changelog automation (PR Bot + Changelog Bot) - ✅ Complete
- [x] Release strategy documentation - ✅ Complete
- [x] Core documentation reorganization - ✅ Complete
- [ ] Tool implementation (ribir-bot release commands, social-card-gen)
- [ ] GitHub Actions workflows
- [ ] System testing and validation
- [ ] Production rollout
