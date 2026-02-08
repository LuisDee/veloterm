# VeloTerm shell integration for Fish
# Source this file in your config.fish:
#   source /path/to/veloterm/shell/fish-integration.fish

# Guard against double-sourcing
if set -q VELOTERM_SHELL_INTEGRATION
    exit 0
end
set -gx VELOTERM_SHELL_INTEGRATION 1

# Track whether a command has been executed
set -g __veloterm_command_started 0

function __veloterm_emit_osc7 --on-variable PWD --description "Report CWD to VeloTerm"
    printf '\e]7;file://%s%s\a' (hostname) "$PWD"
end

function __veloterm_fish_prompt --on-event fish_prompt --description "VeloTerm prompt start"
    set -l exit_status $status
    # Signal command end with exit status (skip on first prompt)
    if test $__veloterm_command_started -eq 1
        printf '\e]133;D;%s\a' $exit_status
        set -g __veloterm_command_started 0
    end
    # Report CWD
    printf '\e]7;file://%s%s\a' (hostname) "$PWD"
    # Signal prompt start
    printf '\e]133;A\a'
end

function __veloterm_fish_preexec --on-event fish_preexec --description "VeloTerm command start"
    set -g __veloterm_command_started 1
    printf '\e]133;B\a'
    printf '\e]133;C\a'
end
