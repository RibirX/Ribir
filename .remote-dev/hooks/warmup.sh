#!/usr/bin/env bash
# Remote-dev warmup hook for Rust projects
# ============================================================================
# This hook runs after the dev shell starts and schedules non-blocking
# background warmup work for the remote workspace.
#
# Current tasks:
# - Restores target/ from cache when available
# - Pushes Rust dependency artifacts to cache when configured
#
# For custom warmup steps:
# - Keep them non-blocking so shell startup stays fast
# - Prefer flake.nix for environment definition and package installation
# - Write progress to .remote-dev/cache-hooks/ or .remote-dev/logs/
#
# Keep this header as operator guidance for future customization.
# ============================================================================

set -u

remote_dev_emit() {
  _rd_msg="$1"
  _rd_color="$2"
  if [ -t 2 ] && [ -z "${NO_COLOR:-}" ] && [ -n "$_rd_color" ]; then
    printf '\033[%sm%s\033[0m\n' "$_rd_color" "$_rd_msg" >&2
  else
    printf '%s\n' "$_rd_msg" >&2
  fi
}

remote_dev_emit_tty() {
  _rd_msg="$1"
  _rd_color="$2"
  [ -w /dev/tty ] || return 0
  if [ -z "${NO_COLOR:-}" ] && [ -n "$_rd_color" ]; then
    printf '\033[%sm%s\033[0m\n' "$_rd_color" "$_rd_msg" >/dev/tty
  else
    printf '%s\n' "$_rd_msg" >/dev/tty
  fi
}

remote_dev_done() {
  _rd_msg="$1"
  _rd_color="$2"
  _rd_emit_mode="${3:-defer}"
  remote_dev_emit "$_rd_msg" "$_rd_color"
  if [ "$_rd_emit_mode" = "now" ]; then
    remote_dev_emit_tty "$_rd_msg" "$_rd_color"
  else
    printf '%s\n' "$_rd_msg" > "${_rd_result_file:?}"
  fi
  : > "${_rd_done_file:?}"
}

remote_dev_write_stamp() {
  _rd_stamp_path="$1"
  _rd_stamp_value="$2"
  _rd_tmp_path="${_rd_stamp_path}.tmp.$$"
  if printf '%s\n' "$_rd_stamp_value" >"$_rd_tmp_path"; then
    mv "$_rd_tmp_path" "$_rd_stamp_path"
  else
    rm -f "$_rd_tmp_path"
  fi
}

remote_dev_is_rust_repo() {
  [ -f Cargo.toml ] || [ -f Cargo.lock ]
}

remote_dev_git_tracks_path() {
  git ls-files --error-unmatch -- "$1" >/dev/null 2>&1
}

remote_dev_needs_rust_flake_snapshot() {
  [ -f Cargo.toml ] && [ ! -f Cargo.lock ] && return 0
  [ -f Cargo.lock ] && ! remote_dev_git_tracks_path Cargo.lock && return 0
  [ -f flake.lock ] && ! remote_dev_git_tracks_path flake.lock && return 0
  return 1
}

remote_dev_run_snapshot_cargo() {
  _rd_snapshot_dir="$1"
  shift

  (
    cd "$_rd_snapshot_dir" || exit 1
    _rd_env_path=".remote-dev-lockfile.env.$$"
    _rd_nix_env_flags="--accept-flake-config --extra-experimental-features 'nix-command flakes' --no-write-lock-file"
    if ! eval nix print-dev-env .#default $_rd_nix_env_flags >"$_rd_env_path"; then
      rm -f "$_rd_env_path"
      exit 1
    fi
    . "$_rd_env_path"
    rm -f "$_rd_env_path"
    export CARGO_HOME="$PWD/.remote-dev/cache/cargo-home"
    mkdir -p "$CARGO_HOME"
    cargo "$@"
  )
}

remote_dev_generate_snapshot_lockfile() {
  _rd_snapshot_dir="$1"
  [ -f "$_rd_snapshot_dir/Cargo.toml" ] || return 0
  [ -f "$_rd_snapshot_dir/Cargo.lock" ] && return 0

  remote_dev_run_snapshot_cargo "$_rd_snapshot_dir" generate-lockfile >/dev/null
}

remote_dev_prepare_rust_flake_ref() {
  if ! remote_dev_needs_rust_flake_snapshot; then
    printf '.\n'
    return 0
  fi

  _rd_snapshot_dir=$(mktemp -d "${TMPDIR:-/tmp}/remote-dev-flake-snapshot.XXXXXX") || return 1

  if ! tar --exclude=.git --exclude=.remote-dev --exclude=target -cf - . 2>/dev/null \
    | tar -xf - -C "$_rd_snapshot_dir" 2>/dev/null; then
    rm -rf "$_rd_snapshot_dir"
    return 1
  fi

  remote_dev_generate_snapshot_lockfile "$_rd_snapshot_dir" || {
    rm -rf "$_rd_snapshot_dir"
    return 1
  }

  printf '%s\n' "$_rd_snapshot_dir"
}

remote_dev_cleanup_flake_ref() {
  [ "$1" = "." ] && return 0
  rm -rf "$1"
}

_rd_warmup_count=0
_rd_warmup_run_id="$$.$RANDOM"

remote_dev_warmup_notice() {
  _rd_task="$1"
  _rd_warmup_count=$((_rd_warmup_count + 1))
  _rd_result_file=".remote-dev/cache-hooks/task-${_rd_warmup_run_id}-${_rd_warmup_count}.result"
  _rd_done_file=".remote-dev/cache-hooks/task-${_rd_warmup_run_id}-${_rd_warmup_count}.done"
  remote_dev_emit "🔄 [remote-dev] $_rd_task" '1;36'
}

remote_dev_warmup_summary() {
  if [ "$_rd_warmup_count" -gt 0 ]; then
    remote_dev_emit "   💡 Background warmup is running (logs: .remote-dev/cache-hooks/)" '0;90'
    (
      _rd_expected="$_rd_warmup_count"
      _rd_dir=".remote-dev/cache-hooks"
      _rd_waited=0
      while [ "$_rd_waited" -lt 300 ]; do
        _rd_count=0
        for _rd_f in "$_rd_dir"/task-${_rd_warmup_run_id}-*.done; do
          [ -f "$_rd_f" ] && _rd_count=$((_rd_count + 1))
        done
        [ "$_rd_count" -ge "$_rd_expected" ] && break
        sleep 1
        _rd_waited=$((_rd_waited + 1))
      done
      for _rd_f in "$_rd_dir"/task-${_rd_warmup_run_id}-*.result; do
        [ -f "$_rd_f" ] || continue
        _rd_msg=$(cat "$_rd_f")
        case "$_rd_msg" in
          ✅*) remote_dev_emit_tty "$_rd_msg" '1;32' ;;
          ❌*) remote_dev_emit_tty "$_rd_msg" '1;31' ;;
          ⏭️*) remote_dev_emit_tty "$_rd_msg" '0;90' ;;
          ⚠️*) remote_dev_emit_tty "$_rd_msg" '1;33' ;;
          *)   remote_dev_emit_tty "$_rd_msg" '0;90' ;;
        esac
        rm -f "$_rd_f"
      done
      rm -f "$_rd_dir"/task-${_rd_warmup_run_id}-*.done
      if [ "$_rd_waited" -ge 300 ]; then
        remote_dev_emit_tty '⚠️  [remote-dev] warmup timed out' '1;33'
      else
        remote_dev_emit_tty '🏁 [remote-dev] warmup complete' '1;36'
      fi
    ) </dev/null >/dev/null 2>&1 &
  fi
}

remote_dev_cache_has_path() {
  _rd_dep_path="$1"
  [ -n "${CACHE_QUERY_URL:-}" ] || return 1
  _rd_dep_name=$(basename "$_rd_dep_path")
  [ -n "$_rd_dep_name" ] || return 1
  _rd_dep_hash="${_rd_dep_name%%-*}"
  [ -n "$_rd_dep_hash" ] || return 1
  _rd_narinfo_url="${CACHE_QUERY_URL%/}/${_rd_dep_hash}.narinfo"

  if command -v curl >/dev/null 2>&1; then
    curl -fsI "$_rd_narinfo_url" >/dev/null 2>&1
  elif command -v wget >/dev/null 2>&1; then
    wget -q --spider "$_rd_narinfo_url" >/dev/null 2>&1
  else
    return 1
  fi
}

remote_dev_restore_deps() {
  remote_dev_is_rust_repo || return 0
  [ ! -e target ] || return 0
  _rd_log_path=".remote-dev/cache-hooks/deps-restore.log"
  remote_dev_warmup_notice 'Restoring target/ from cache'

  (
    _rd_flake_ref=$(remote_dev_prepare_rust_flake_ref) || {
      remote_dev_done '❌ [remote-dev] rust flake snapshot failed' '1;31' now
      return 0
    }
    _rd_nix_flags="--accept-flake-config --extra-experimental-features 'nix-command flakes' --no-write-lock-file --no-link"
    _rd_stdout_path=".remote-dev/cache-hooks/deps-restore.stdout.$$"
    _rd_dep_path=""
    if eval nix build "${_rd_flake_ref}#dev-cargo-deps" $_rd_nix_flags ${REMOTE_DEV_DEPS_NIX_OPTIONS:-} --print-out-paths >"$_rd_stdout_path"; then
      _rd_dep_path=$(cat "$_rd_stdout_path")
    else
      _rd_status=$?
      rm -f "$_rd_stdout_path"
      remote_dev_cleanup_flake_ref "$_rd_flake_ref"
      remote_dev_done "❌ [remote-dev] deps restore build failed (exit code $_rd_status)" '1;31' now
      return 0
    fi
    rm -f "$_rd_stdout_path"
    remote_dev_cleanup_flake_ref "$_rd_flake_ref"

    if [ -z "$_rd_dep_path" ]; then
      remote_dev_done '⏭️  [remote-dev] no cached target/ found' '0;90' now
      return 0
    fi
    if [ -e target ]; then
      remote_dev_done '⚠️  [remote-dev] target/ already exists' '1;33' now
      return 0
    fi

    remote_dev_emit '⚡ [remote-dev] deps restore started' '1;36'

    mkdir -p .remote-dev || return 0
    _rd_seed_dir=".remote-dev/target-seed.$$.$RANDOM"
    mkdir -p "$_rd_seed_dir" || return 0
    _rd_restore_ok=0

    if [ -f "$_rd_dep_path/target.tar.zst" ]; then
      if tar --zstd -xf "$_rd_dep_path/target.tar.zst" -C "$_rd_seed_dir" 2>/dev/null; then
        _rd_restore_ok=1
      fi
    else
      if cp -R "$_rd_dep_path"/. "$_rd_seed_dir"/ 2>/dev/null; then
        _rd_restore_ok=1
      fi
    fi

    chmod -R u+w "$_rd_seed_dir" 2>/dev/null || true

    if [ "$_rd_restore_ok" != 1 ]; then
      rm -rf "$_rd_seed_dir"
      remote_dev_done '❌ [remote-dev] target/ restore failed' '1;31' now
    elif [ -e target ]; then
      rm -rf "$_rd_seed_dir"
      remote_dev_done '⚠️  [remote-dev] target/ already exists' '1;33' now
    elif mv "$_rd_seed_dir" target 2>/dev/null; then
      remote_dev_done '✅ [remote-dev] target/ restored from cache' '1;32' now
    else
      rm -rf "$_rd_seed_dir"
      if [ -e target ]; then
        remote_dev_done '⚠️  [remote-dev] target/ already exists' '1;33' now
      else
        remote_dev_done '❌ [remote-dev] target/ restore failed' '1;31' now
      fi
    fi
  ) </dev/null >>"$_rd_log_path" 2>&1 &
}

remote_dev_push_deps() {
  remote_dev_is_rust_repo || return 0
  [ -n "${CACHE_PUSH_CMD:-}" ] || return 0
  _rd_log_path=".remote-dev/cache-hooks/deps-push.log"
  remote_dev_warmup_notice 'Pushing .#dev-cargo-deps to cache'

  (
    _rd_flake_ref=$(remote_dev_prepare_rust_flake_ref) || {
      remote_dev_done '❌ [remote-dev] rust flake snapshot failed' '1;31'
      return 0
    }
    _rd_dep_target="${_rd_flake_ref}#dev-cargo-deps"
    _rd_nix_flags="--accept-flake-config --extra-experimental-features 'nix-command flakes' --no-write-lock-file --no-link"
    _rd_stdout_path=".remote-dev/cache-hooks/deps-push.stdout.$$"
    _rd_stderr_path=".remote-dev/cache-hooks/deps-push.stderr.$$"
    _rd_dep_path=""
    if eval nix build "$_rd_dep_target" $_rd_nix_flags ${REMOTE_DEV_DEPS_NIX_OPTIONS:-} --print-out-paths >"$_rd_stdout_path" 2>"$_rd_stderr_path"; then
      _rd_dep_path=$(cat "$_rd_stdout_path")
    else
      _rd_status=$?
      if grep -q "does not provide attribute 'packages\\..*\\.dev-cargo-deps'" "$_rd_stderr_path" \
        || grep -q "does not provide attribute 'legacyPackages\\..*\\.dev-cargo-deps'" "$_rd_stderr_path" \
        || grep -q "does not provide attribute 'dev-cargo-deps'" "$_rd_stderr_path"; then
        cat "$_rd_stderr_path"
        remote_dev_done '⚠️  [remote-dev] skipping deps cache push: flake does not expose .#dev-cargo-deps' '1;33'
      else
        cat "$_rd_stderr_path"
        remote_dev_done "❌ [remote-dev] deps build failed (exit code $_rd_status)" '1;31'
      fi
      remote_dev_cleanup_flake_ref "$_rd_flake_ref"
      rm -f "$_rd_stdout_path" "$_rd_stderr_path"
      return 0
    fi
    remote_dev_cleanup_flake_ref "$_rd_flake_ref"
    rm -f "$_rd_stdout_path" "$_rd_stderr_path"

    if [ -z "$_rd_dep_path" ]; then
      remote_dev_done '⏭️  [remote-dev] no deps build output to push' '0;90'
      return 0
    fi

    _rd_stamp_path=".remote-dev/cache-hooks/deps.path"
    _rd_last_pushed=$(cat "$_rd_stamp_path" 2>/dev/null || true)
    if [ "$_rd_dep_path" = "$_rd_last_pushed" ]; then
      remote_dev_done '⏭️  [remote-dev] deps cache already up to date' '0;90'
      return 0
    fi
    if remote_dev_cache_has_path "$_rd_dep_path"; then
      remote_dev_write_stamp "$_rd_stamp_path" "$_rd_dep_path"
      remote_dev_done '⏭️  [remote-dev] deps cache already contains .#dev-cargo-deps' '0;90'
      return 0
    fi

    remote_dev_emit "⚡ [remote-dev] cache push started: $_rd_dep_target" '1;36'
    if eval $CACHE_PUSH_CMD "$_rd_dep_path" >/dev/null 2>&1; then
      remote_dev_write_stamp "$_rd_stamp_path" "$_rd_dep_path"
      remote_dev_done "✅ [remote-dev] $_rd_dep_target pushed to cache" '1;32'
    else
      remote_dev_done "❌ [remote-dev] $_rd_dep_target push failed" '1;31'
    fi
  ) </dev/null >>"$_rd_log_path" 2>&1 &
}

mkdir -p .remote-dev/cache-hooks
[ -f "$HOME/.cache-meta.env" ] && . "$HOME/.cache-meta.env"

remote_dev_restore_deps
remote_dev_push_deps
remote_dev_warmup_summary
