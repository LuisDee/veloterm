# VeloTerm shell integration for Bash
# Source this file in your .bashrc:
#   source /path/to/veloterm/shell/bash-integration.sh

# Guard against double-sourcing
if [[ -n "$VELOTERM_SHELL_INTEGRATION" ]]; then
    return 0
fi
export VELOTERM_SHELL_INTEGRATION=1

# Emit OSC 7 with the current working directory
__veloterm_osc7() {
    printf '\e]7;file://%s%s\a' "${HOSTNAME}" "${PWD}"
}

# Emit OSC 133;A (prompt start)
__veloterm_prompt_start() {
    printf '\e]133;A\a'
}

# Emit OSC 133;B (command start — user pressed Enter)
__veloterm_command_start() {
    printf '\e]133;B\a'
}

# Emit OSC 133;C (command output start)
__veloterm_command_output() {
    printf '\e]133;C\a'
}

# Emit OSC 133;D (command end with exit status)
__veloterm_command_end() {
    printf '\e]133;D;%s\a' "$1"
}

# Preserve existing PROMPT_COMMAND
__veloterm_original_prompt_command="${PROMPT_COMMAND}"

__veloterm_precmd() {
    local exit_status=$?
    # Signal command end with exit status (skip on first prompt)
    if [[ -n "$__veloterm_command_started" ]]; then
        __veloterm_command_end "$exit_status"
        unset __veloterm_command_started
    fi
    # Report CWD
    __veloterm_osc7
    # Signal prompt start
    __veloterm_prompt_start
    # Run original PROMPT_COMMAND if set
    if [[ -n "$__veloterm_original_prompt_command" ]]; then
        eval "$__veloterm_original_prompt_command"
    fi
}

# Use DEBUG trap to detect command execution
__veloterm_preexec() {
    if [[ -z "$__veloterm_command_started" ]]; then
        __veloterm_command_started=1
        __veloterm_command_start
        __veloterm_command_output
    fi
}

PROMPT_COMMAND="__veloterm_precmd"
trap '__veloterm_preexec' DEBUG
