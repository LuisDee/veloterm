# VeloTerm shell integration for Zsh
# Source this file in your .zshrc:
#   source /path/to/veloterm/shell/zsh-integration.sh

# Guard against double-sourcing
if [[ -n "$VELOTERM_SHELL_INTEGRATION" ]]; then
    return 0
fi
export VELOTERM_SHELL_INTEGRATION=1

# Emit OSC 7 with the current working directory
__veloterm_osc7() {
    printf '\e]7;file://%s%s\a' "${HOST}" "${PWD}"
}

# precmd: runs before each prompt is displayed
__veloterm_precmd() {
    local exit_status=$?
    # Signal command end with exit status (skip on first prompt)
    if [[ -n "$__veloterm_command_started" ]]; then
        printf '\e]133;D;%s\a' "$exit_status"
        unset __veloterm_command_started
    fi
    # Report CWD
    __veloterm_osc7
    # Signal prompt start
    printf '\e]133;A\a'
}

# preexec: runs after user presses Enter but before the command executes
__veloterm_preexec() {
    __veloterm_command_started=1
    printf '\e]133;B\a'
    printf '\e]133;C\a'
}

# Append to existing hooks (don't replace)
autoload -Uz add-zsh-hook
add-zsh-hook precmd __veloterm_precmd
add-zsh-hook preexec __veloterm_preexec
