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

**2. [Social Card Generation](03-social-card-generation.md)**
   - Standalone ribir-render tool design
   - Visual asset generation for releases

---

## Automation Tools

### Ribir Bot (`tools/ribir-bot`)

Unified CLI tool for PR, changelog, and release automation.

> **Reference:** For installation, usage, and full command reference, see [tools/ribir-bot/README.md](../../tools/ribir-bot/README.md).

### GitHub Actions Workflows

**Release Alpha** (automated weekly)
- Runs every Tuesday at 14:00 UTC
- Auto-increments version (e.g., 0.5.0-alpha.23 → alpha.24)
- Collects changelog entries and creates GitHub Release
- Publishes to crates.io
- Can be triggered manually for on-demand releases

**Stable Preparation** (`release enter-rc`, manual)
- Creates release branch (`release-0.5.x`) from master
- Archives and merges all alpha changelogs
- AI generates highlights in PR body (editable)
- Creates PR (`release-0.5.x` → `master`) for human review
- **Automatically publishes RC.1**

**Release RC** (manual, for rc.2+ only)
- Runs on release branch when critical bugs are fixed during RC testing
- Publishes rc.2, rc.3, etc.
- Creates GitHub Release (marked as pre-release)

**Release Stable** (`@ribir-bot release-stable`, bot-driven)
- Triggered by `@ribir-bot release-stable` command in preparation PR
- Merges any bug fix changelog from RC versions
- Extracts highlights from PR body and writes to CHANGELOG.md
- Publishes to crates.io and creates GitHub Release (stable)
- Automatically merges the preparation PR to master

**Release Patch** (manual)
- Quick bug fix releases on release branch
- Collects only bug fix changelog entries
- No social cards or highlights (not needed for patches)
- Immediate publication

## Quick Start

**New to the release system?**

1. **[Release Strategy](00-release-strategy.md)** - Understand the overall flow
2. **[Changelog Automation](01-changelog-automation.md)** - Learn how changelog works and release materials
3. **[Social Card Generation](03-social-card-generation.md)** - Understand ribir-render tooling

**Need technical details?**
- See [tools/ribir-bot/README.md](../../tools/ribir-bot/README.md) for CLI command reference.

## Related Documentation

- [Release Strategy](00-release-strategy.md) - Release flow and versioning
- [Changelog Automation](01-changelog-automation.md) - PR bot and changelog automation

- [PR Template](../../../.github/pull_request_template.md) - PR workflow

## Status

- [x] Changelog automation (PR Bot + Changelog Bot) - ✅ Complete
- [x] Release strategy documentation - ✅ Complete
- [x] Core documentation reorganization - ✅ Complete
- [x] Tool implementation (ribir-bot release commands) - ✅ Complete
- [x] GitHub Actions workflows - ✅ Complete
- [ ] Social card generation (ribir-render)
- [ ] System testing and validation
- [ ] Production rollout
