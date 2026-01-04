# Complete Release Flow - Operations Handbook

**Topic:** Step-by-step operational procedures for each release type
**Status:** Handbook
**Audience:** Maintainers and release managers

---

## Overview

This document provides concrete operational steps for executing each release type. For the strategic rationale behind these release types, see [00-release-strategy.md](00-release-strategy.md).

**Quick Navigation:**
- [Unified Release Command](#unified-release-command)
- [Alpha Release Operations](#alpha-release-operations)
- [RC Release Operations](#rc-release-operations)
- [Stable Release Operations](#stable-release-operations)
- [Patch Release Operations](#patch-release-operations)

---

## Unified Release Command

All releases now use the unified `ribir-bot release next` command:

```bash
# Preview what will happen (default - dry-run mode)
ribir-bot release next <level>

# Execute the release
ribir-bot release next <level> --execute
```

**Available levels:**

| Level | Example | Use Case |
|-------|---------|----------|
| `alpha` | 0.5.0-alpha.54 | Weekly development releases |
| `rc` | 0.5.0-rc.1 | Release candidates |
| `patch` | 0.5.1 | Bug fix releases |
| `minor` | 0.6.0 | Feature releases |
| `major` | 1.0.0 | Major version releases |

**What the command does:**

1. **Get version**: Runs `cargo release <level> --dry-run` to determine next version
2. **Collect changelog**: Gathers entries from merged PRs into CHANGELOG.md
3. **Cargo release**: Bumps versions, commits, tags, pushes, publishes to crates.io
4. **GitHub Release**: Creates release with notes from changelog

**Dry-run mode** (default): Shows changelog and release notes preview without making changes.

---

## Workflow Triggers

All release workflows are GitHub Actions that can be triggered in two ways:

**Automatic (scheduled or event-based):**
- Runs automatically based on schedule (e.g., weekly) or events (e.g., PR merge)
- No manual action required

**Manual (workflow_dispatch):**
- Triggered manually via GitHub Actions UI or CLI
- May require input parameters (e.g., version number)

Each operation section below specifies which trigger type applies.

---

## Alpha Release Operations

**When:** Weekly automatic (Tuesdays 14:00 UTC) or manual trigger after major PR merge

**Prerequisites:**
- Master branch is in working state
- Tests are passing
- Recent PRs have changelog entries

**Trigger:** Automatic (Tuesdays 14:00 UTC) or Manual

**Command:**
```bash
# Preview
ribir-bot release next alpha

# Execute
ribir-bot release next alpha --execute
```

**What happens:**
1. Gets next alpha version from cargo-release (0.5.0-alpha.23 → 0.5.0-alpha.24)
2. Collects changelog entries from merged PRs
3. Updates CHANGELOG.md
4. Runs `cargo release alpha` (version bump, commit, tag, push, publish)
5. Creates GitHub Release (pre-release flag)

**Verification Checklist:**
- [ ] GitHub Release appears on releases page
- [ ] Version number incremented correctly
- [ ] CHANGELOG.md updated with new section
- [ ] Release marked as "Pre-release"
- [ ] Package published to crates.io

**Timeline:** ~2-5 minutes

**See also:** [00-release-strategy.md - Alpha Releases](00-release-strategy.md#1-alpha-releases-050-alphax)

---

## RC Release Operations

**When:** All planned features complete, ready to stabilize

**Prerequisites:**
- All target features merged to master
- Alpha testing shows stability
- Team consensus to move to RC

### Phase 1: Preparation

**Trigger:** Manual (version input: `0.5.0`)

**Command:**
```bash
ribir-bot release prepare --version 0.5.0
```

**What happens automatically:**
1. Collects all alpha changelogs
2. AI analyzes and generates highlights section in CHANGELOG.md
3. Generates social card preview (PNG) - future
4. Creates PR: "Release 0.5.0-rc.1 Preparation"
5. Uploads artifacts for review (future)

**Timeline:** ~5-10 minutes

### Phase 2: Review and Approval

**Download and review artifacts:**
1. Go to PR created by workflow
2. Review CHANGELOG.md for highlights section
3. When social cards are implemented:
   - Navigate to Actions tab → Find workflow run
   - Scroll to "Artifacts" section
   - Download "release-materials" ZIP
   - Extract and review `social-card-preview.png`

**Review checklist:**
- [ ] All important PRs included in changelog
- [ ] Highlights section in CHANGELOG.md makes sense (3-5 items)
- [ ] Social card is readable and accurate (when implemented)
- [ ] Version number and date correct

**If changes needed:**

Edit `CHANGELOG.md` directly in the preparation PR branch to adjust the highlights or fix any issues.

### Phase 3: Publishing

**Trigger:** Automatic (on PR merge) or Manual

**Command (manual):**
```bash
ribir-bot release next rc --execute
```

**What happens automatically:**
1. Creates release branch `release-0.5.x`
2. Runs `cargo release rc` (version bump, commit, tag, push, publish)
3. Creates GitHub Release v0.5.0-rc.1
4. Comments on PR with release link
5. (Note: Social cards are NOT attached to RC releases)

**Timeline:** ~3-5 minutes

**Verification Checklist:**
- [ ] Release branch `release-0.5.x` created
- [ ] GitHub Release published with pre-release flag
- [ ] Social card reviewed in PR
- [ ] CHANGELOG.md updated on master with highlights section

**See also:** [00-release-strategy.md - Release Candidate](00-release-strategy.md#2-release-candidate-050-rc1)

---

## Stable Release Operations

**When:** After RC testing period (1-2 weeks), no critical bugs

**Prerequisites:**
- RC testing complete (1-2 weeks)
- No critical bugs reported
- Community feedback positive (or no feedback after 2 weeks)

**Trigger:** Manual

**Command:**
```bash
# Preview
ribir-bot release promote --version 0.5.0

# Execute
ribir-bot release promote --version 0.5.0 --execute
```

**What happens:**

1. Collects changelog from all RC versions (rc.2, rc.3 if multiple exist)
2. Merges bug fix entries into stable changelog
3. Reuses RC.1 materials (highlights in CHANGELOG, social card)
4. Runs `cargo release 0.5.0` (version bump, commit, tag, push, publish)
5. Publishes GitHub Release (removes pre-release flag)

**Note:** RC versions only fix bugs and never add new features. Therefore, highlights and social cards generated during RC.1 preparation are always reused for stable release, regardless of how many RC versions exist.

**Verification Checklist:**
- [ ] GitHub Release v0.5.0 published (stable, not pre-release)
- [ ] Social card attached (when implemented)
- [ ] CHANGELOG.md contains highlights section (from RC.1)
- [ ] Pre-release flag removed
- [ ] Package published to crates.io

**Timeline:** ~2-5 minutes

**Post-Release:**
- Monitor community feedback
- Share release on social media (optional)

**See also:** [00-release-strategy.md - Stable Release](00-release-strategy.md#3-stable-release-050)

---

## Patch Release Operations

**When:** Critical bugs in stable, security fixes needed

**Prerequisites:**
- Bug fix PRs merged to release branch
- Fixes are tested
- Ready to publish immediately

**Trigger:** Manual

**Command:**
```bash
# Preview
ribir-bot release next patch

# Execute
ribir-bot release next patch --execute
```

**What happens:**
1. Gets next patch version (0.5.0 → 0.5.1)
2. Collects changelog entries from bug fix PRs
3. Updates changelog on release branch
4. Runs `cargo release patch` (version bump, commit, tag, push, publish)
5. Creates GitHub Release

**Materials generated:**
- ✅ Updated CHANGELOG entry
- ✅ GitHub Release with notes
- ❌ No social cards or highlights (not needed for patches)

**Verification Checklist:**
- [ ] GitHub Release appears
- [ ] Changelog updated on release branch
- [ ] Version number correct
- [ ] Package published to crates.io

**Timeline:** ~2-3 minutes

**See also:** [00-release-strategy.md - Patch Releases](00-release-strategy.md#4-patch-releases-051-052)

---

## Related Documentation

- **[00-release-strategy.md](00-release-strategy.md)** - Why we have these release types
- **[01-changelog-automation.md](01-changelog-automation.md)** - How PR Bot and Changelog Bot work, release materials
- **[03-social-card-generation.md](03-social-card-generation.md)** - Social card tooling details
