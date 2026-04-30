#!/usr/bin/env bash
# Remote-dev exit hook
# ============================================================================
# This hook runs when the remote workspace session ends.
# It cleans up temporary files and partial state before shutdown.
#
# Current tasks:
# - Removes temporary target-seed directories
# - Cleans up temporary .tmp.* files in cache-hooks
#
# For custom cleanup:
# - Add commands before the cleanup operations
# - Keep cleanup idempotent so repeated runs stay safe
# - Avoid deleting durable user data from the workspace
#
# Keep this header as operator guidance for future customization.
# ============================================================================

set -u

find .remote-dev -maxdepth 1 -type d -name 'target-seed.*' -exec rm -rf {} + 2>/dev/null || true
find .remote-dev/cache-hooks -maxdepth 1 -type f -name '*.tmp.*' -delete 2>/dev/null || true
