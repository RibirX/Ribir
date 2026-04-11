#!/usr/bin/env bash
# Remote-dev startup hook
# ============================================================================
# This hook runs after the remote workspace is prepared and the shell starts.
# It performs lightweight setup that should always stay fast.
#
# Current tasks:
# - Creates cache helper and log directories
# - Loads cache metadata when it is available
#
# For custom startup steps:
# - Add them before the final status message
# - Keep the final status message as the last operation
# - Prefer flake.nix for environment definition and package installation
#
# Keep this header as operator guidance for future customization.
# ============================================================================

set -u

mkdir -p .remote-dev/cache-hooks .remote-dev/logs
[ -f "$HOME/.cache-meta.env" ] && . "$HOME/.cache-meta.env"

if [ -t 2 ] && [ -z "${NO_COLOR:-}" ]; then
  printf '\033[1;36m%s\033[0m\n' \
    "🔄 [remote-dev] startup complete; background warmup may continue in .remote-dev/logs/warmup.log" >&2
else
  printf '%s\n' \
    "[remote-dev] startup complete; background warmup may continue in .remote-dev/logs/warmup.log" >&2
fi
