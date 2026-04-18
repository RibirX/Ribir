# Remote workspace Zsh configuration.
# Edit this file freely; plugin paths (@PLACEHOLDER@) are substituted
# automatically by Nix on `remote-dev up`.

export ZDOTDIR="$HOME"
export HISTFILE="$HOME/.local/state/remote-dev/history/Ribir.zsh_history"
export HISTSIZE=50000
export SAVEHIST=50000
export EDITOR="${EDITOR:-vim}"

# File-level history sync works best with zsh's fcntl lock path.
setopt HIST_FCNTL_LOCK
setopt APPEND_HISTORY
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_SPACE
setopt SHARE_HISTORY

# remote-dev lands in zsh via "ssh -> bash -> exec zsh", so Ghostty's
# auto-detected shell integration may not be injected for the final shell.
if [[ -n ${GHOSTTY_RESOURCES_DIR:-} ]]; then
  _ghostty_integration="$GHOSTTY_RESOURCES_DIR/shell-integration/zsh/ghostty-integration"
  [[ -r "$_ghostty_integration" ]] && source "$_ghostty_integration"
  unset _ghostty_integration
fi

autoload -Uz compinit
compinit

if [ -f @ZSH_AUTOSUGGESTIONS@ ]; then
  source @ZSH_AUTOSUGGESTIONS@
fi

if command -v starship >/dev/null 2>&1; then
  export STARSHIP_CONFIG="$HOME/.config/starship.toml"
  eval "$(starship init zsh)"
fi

if [ -f @ZSH_SYNTAX_HIGHLIGHTING@ ]; then
  source @ZSH_SYNTAX_HIGHLIGHTING@
fi

if [ -f @ZSH_HISTORY_SUBSTRING_SEARCH@ ]; then
  source @ZSH_HISTORY_SUBSTRING_SEARCH@
  HISTORY_SUBSTRING_SEARCH_ENSURE_UNIQUE=1

  if [[ -n ${terminfo[kcuu1]:-} && -n ${terminfo[kcud1]:-} ]]; then
    bindkey "${terminfo[kcuu1]}" history-substring-search-up
    bindkey "${terminfo[kcud1]}" history-substring-search-down
  fi

  bindkey '^[[A' history-substring-search-up
  bindkey '^[[B' history-substring-search-down
fi

remote_dev_set_cursor() {
  [[ -t 1 ]] || return 0
  case "${1:-beam}" in
    beam) printf '\033[6 q' ;;
    block) printf '\033[2 q' ;;
  esac
}

autoload -Uz add-zsh-hook
remote_dev_set_cursor_beam() { remote_dev_set_cursor beam; }
remote_dev_set_cursor_block() { remote_dev_set_cursor block; }
remote_dev_flush_history() {
  [[ -n ${HISTFILE:-} ]] || return 0
  fc -W "$HISTFILE" >/dev/null 2>&1 || true
}
add-zsh-hook precmd remote_dev_set_cursor_beam
add-zsh-hook preexec remote_dev_set_cursor_block
add-zsh-hook zshexit remote_dev_flush_history

if [[ -o zle ]]; then
  function zle-line-init() {
    remote_dev_set_cursor beam
  }
  zle -N zle-line-init
fi

alias ll='ls -lah'
alias gs='git status --short'

# remote-dev shell marker: this file is synced from .remote-dev/.zshrc
