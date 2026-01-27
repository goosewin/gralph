#compdef gralph

autoload -U is-at-least

_gralph() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_gralph_commands" \
"*::: :->gralph" \
&& ret=0
    case $state in
    (gralph)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
'-n+[Session name (default\: directory name)]:NAME:_default' \
'--name=[Session name (default\: directory name)]:NAME:_default' \
'--max-iterations=[Max iterations before giving up (default\: 30)]:MAX_ITERATIONS:_default' \
'-f+[Task file path (default\: PRD.md)]:TASK_FILE:_default' \
'--task-file=[Task file path (default\: PRD.md)]:TASK_FILE:_default' \
'--completion-marker=[Completion promise text (default\: COMPLETE)]:COMPLETION_MARKER:_default' \
'-b+[AI backend (default\: claude)]:BACKEND:_default' \
'--backend=[AI backend (default\: claude)]:BACKEND:_default' \
'-m+[Model override (format depends on backend)]:MODEL:_default' \
'--model=[Model override (format depends on backend)]:MODEL:_default' \
'--variant=[Model variant override (backend-specific)]:VARIANT:_default' \
'--prompt-template=[Path to custom prompt template file]:PROMPT_TEMPLATE:_files' \
'--webhook=[Notification webhook URL]:WEBHOOK:_default' \
'--no-worktree[Disable automatic worktree creation]' \
'--no-tmux[Run in foreground (blocks)]' \
'--strict-prd[Validate PRD before starting the loop]' \
'-h[Print help]' \
'--help[Print help]' \
':dir -- Project directory to run the loop in:_files' \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
'-a[Stop all loops]' \
'--all[Stop all loops]' \
'-h[Print help]' \
'--help[Print help]' \
'::name -- Session name:_default' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(logs)
_arguments "${_arguments_options[@]}" : \
'--follow[Follow log output]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Session name:_default' \
&& ret=0
;;
(resume)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
'::name -- Session name:_default' \
&& ret=0
;;
(init)
_arguments "${_arguments_options[@]}" : \
'--dir=[Target directory (default\: current)]:DIR:_files' \
'--force[Overwrite existing files]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(prd)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_gralph__prd_commands" \
"*::: :->prd" \
&& ret=0

    case $state in
    (prd)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-prd-command-$line[1]:"
        case $line[1] in
            (check)
_arguments "${_arguments_options[@]}" : \
'--allow-missing-context[Allow missing Context Bundle paths]' \
'-h[Print help]' \
'--help[Print help]' \
':file -- PRD file to validate:_files' \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
'--dir=[Project directory (default\: current)]:DIR:_files' \
'-o+[Output PRD file path (default\: PRD.generated.md)]:OUTPUT:_files' \
'--output=[Output PRD file path (default\: PRD.generated.md)]:OUTPUT:_files' \
'--goal=[Short description of what to build]:GOAL:_default' \
'--constraints=[Constraints or non-functional requirements]:CONSTRAINTS:_default' \
'--context=[Extra context files (comma-separated)]:CONTEXT:_default' \
'--sources=[External URLs or references (comma-separated)]:SOURCES:_default' \
'-b+[Backend for PRD generation (default\: config/default)]:BACKEND:_default' \
'--backend=[Backend for PRD generation (default\: config/default)]:BACKEND:_default' \
'-m+[Model override for PRD generation]:MODEL:_default' \
'--model=[Model override for PRD generation]:MODEL:_default' \
'--variant=[Model variant override (backend-specific)]:VARIANT:_default' \
'--allow-missing-context[Allow missing Context Bundle paths]' \
'--multiline[Enable multiline prompts (interactive)]' \
'(--interactive)--no-interactive[Disable interactive prompts]' \
'(--no-interactive)--interactive[Force interactive prompts]' \
'--force[Overwrite existing output file]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__prd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-prd-help-command-$line[1]:"
        case $line[1] in
            (check)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(worktree)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_gralph__worktree_commands" \
"*::: :->worktree" \
&& ret=0

    case $state in
    (worktree)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-worktree-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':id -- Task ID (e.g. C-1):_default' \
&& ret=0
;;
(finish)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':id -- Task ID (e.g. C-1):_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__worktree__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-worktree-help-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(finish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(backends)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_gralph__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-config-command-$line[1]:"
        case $line[1] in
            (get)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':key -- Config key:_default' \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':key -- Config key:_default' \
':value -- Config value:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__config__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-config-help-command-$line[1]:"
        case $line[1] in
            (get)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(server)
_arguments "${_arguments_options[@]}" : \
'-H+[Host/IP to bind to (default\: 127.0.0.1)]:HOST:_default' \
'--host=[Host/IP to bind to (default\: 127.0.0.1)]:HOST:_default' \
'-p+[Port number (default\: 8080)]:PORT:_default' \
'--port=[Port number (default\: 8080)]:PORT:_default' \
'-t+[Authentication token (required for non-localhost)]:TOKEN:_default' \
'--token=[Authentication token (required for non-localhost)]:TOKEN:_default' \
'--open[Disable token requirement (use with caution)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(version)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(run-loop)
_arguments "${_arguments_options[@]}" : \
'--name=[Session name]:NAME:_default' \
'--max-iterations=[]:MAX_ITERATIONS:_default' \
'--task-file=[]:TASK_FILE:_default' \
'--completion-marker=[]:COMPLETION_MARKER:_default' \
'--backend=[]:BACKEND:_default' \
'--model=[]:MODEL:_default' \
'--variant=[]:VARIANT:_default' \
'--prompt-template=[]:PROMPT_TEMPLATE:_files' \
'--webhook=[]:WEBHOOK:_default' \
'--no-worktree[Disable automatic worktree creation]' \
'--strict-prd[]' \
'-h[Print help]' \
'--help[Print help]' \
':dir:_files' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-help-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(logs)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(resume)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(prd)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__help__prd_commands" \
"*::: :->prd" \
&& ret=0

    case $state in
    (prd)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-help-prd-command-$line[1]:"
        case $line[1] in
            (check)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(worktree)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__help__worktree_commands" \
"*::: :->worktree" \
&& ret=0

    case $state in
    (worktree)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-help-worktree-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(finish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(backends)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
":: :_gralph__help__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:gralph-help-config-command-$line[1]:"
        case $line[1] in
            (get)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(server)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(version)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(run-loop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_gralph_commands] )) ||
_gralph_commands() {
    local commands; commands=(
'start:Start a new gralph loop' \
'stop:Stop a running loop' \
'status:Show status of all loops' \
'logs:View logs for a loop' \
'resume:Resume crashed/stopped loops' \
'init:Initialize shared context files' \
'prd:Generate or validate PRDs' \
'worktree:Manage task worktrees' \
'backends:List available AI backends' \
'config:Manage configuration' \
'server:Start status API server' \
'version:Show version' \
'update:Install the latest release' \
'run-loop:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph commands' commands "$@"
}
(( $+functions[_gralph__backends_commands] )) ||
_gralph__backends_commands() {
    local commands; commands=()
    _describe -t commands 'gralph backends commands' commands "$@"
}
(( $+functions[_gralph__config_commands] )) ||
_gralph__config_commands() {
    local commands; commands=(
'get:Get config value' \
'set:Set config value' \
'list:List config values' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph config commands' commands "$@"
}
(( $+functions[_gralph__config__get_commands] )) ||
_gralph__config__get_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config get commands' commands "$@"
}
(( $+functions[_gralph__config__help_commands] )) ||
_gralph__config__help_commands() {
    local commands; commands=(
'get:Get config value' \
'set:Set config value' \
'list:List config values' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph config help commands' commands "$@"
}
(( $+functions[_gralph__config__help__get_commands] )) ||
_gralph__config__help__get_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config help get commands' commands "$@"
}
(( $+functions[_gralph__config__help__help_commands] )) ||
_gralph__config__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config help help commands' commands "$@"
}
(( $+functions[_gralph__config__help__list_commands] )) ||
_gralph__config__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config help list commands' commands "$@"
}
(( $+functions[_gralph__config__help__set_commands] )) ||
_gralph__config__help__set_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config help set commands' commands "$@"
}
(( $+functions[_gralph__config__list_commands] )) ||
_gralph__config__list_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config list commands' commands "$@"
}
(( $+functions[_gralph__config__set_commands] )) ||
_gralph__config__set_commands() {
    local commands; commands=()
    _describe -t commands 'gralph config set commands' commands "$@"
}
(( $+functions[_gralph__help_commands] )) ||
_gralph__help_commands() {
    local commands; commands=(
'start:Start a new gralph loop' \
'stop:Stop a running loop' \
'status:Show status of all loops' \
'logs:View logs for a loop' \
'resume:Resume crashed/stopped loops' \
'init:Initialize shared context files' \
'prd:Generate or validate PRDs' \
'worktree:Manage task worktrees' \
'backends:List available AI backends' \
'config:Manage configuration' \
'server:Start status API server' \
'version:Show version' \
'update:Install the latest release' \
'run-loop:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph help commands' commands "$@"
}
(( $+functions[_gralph__help__backends_commands] )) ||
_gralph__help__backends_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help backends commands' commands "$@"
}
(( $+functions[_gralph__help__config_commands] )) ||
_gralph__help__config_commands() {
    local commands; commands=(
'get:Get config value' \
'set:Set config value' \
'list:List config values' \
    )
    _describe -t commands 'gralph help config commands' commands "$@"
}
(( $+functions[_gralph__help__config__get_commands] )) ||
_gralph__help__config__get_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help config get commands' commands "$@"
}
(( $+functions[_gralph__help__config__list_commands] )) ||
_gralph__help__config__list_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help config list commands' commands "$@"
}
(( $+functions[_gralph__help__config__set_commands] )) ||
_gralph__help__config__set_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help config set commands' commands "$@"
}
(( $+functions[_gralph__help__help_commands] )) ||
_gralph__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help help commands' commands "$@"
}
(( $+functions[_gralph__help__init_commands] )) ||
_gralph__help__init_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help init commands' commands "$@"
}
(( $+functions[_gralph__help__logs_commands] )) ||
_gralph__help__logs_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help logs commands' commands "$@"
}
(( $+functions[_gralph__help__prd_commands] )) ||
_gralph__help__prd_commands() {
    local commands; commands=(
'check:Validate PRD task blocks' \
'create:Generate a spec-compliant PRD' \
    )
    _describe -t commands 'gralph help prd commands' commands "$@"
}
(( $+functions[_gralph__help__prd__check_commands] )) ||
_gralph__help__prd__check_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help prd check commands' commands "$@"
}
(( $+functions[_gralph__help__prd__create_commands] )) ||
_gralph__help__prd__create_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help prd create commands' commands "$@"
}
(( $+functions[_gralph__help__resume_commands] )) ||
_gralph__help__resume_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help resume commands' commands "$@"
}
(( $+functions[_gralph__help__run-loop_commands] )) ||
_gralph__help__run-loop_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help run-loop commands' commands "$@"
}
(( $+functions[_gralph__help__server_commands] )) ||
_gralph__help__server_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help server commands' commands "$@"
}
(( $+functions[_gralph__help__start_commands] )) ||
_gralph__help__start_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help start commands' commands "$@"
}
(( $+functions[_gralph__help__status_commands] )) ||
_gralph__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help status commands' commands "$@"
}
(( $+functions[_gralph__help__stop_commands] )) ||
_gralph__help__stop_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help stop commands' commands "$@"
}
(( $+functions[_gralph__help__update_commands] )) ||
_gralph__help__update_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help update commands' commands "$@"
}
(( $+functions[_gralph__help__version_commands] )) ||
_gralph__help__version_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help version commands' commands "$@"
}
(( $+functions[_gralph__help__worktree_commands] )) ||
_gralph__help__worktree_commands() {
    local commands; commands=(
'create:Create task worktree' \
'finish:Finish task worktree' \
    )
    _describe -t commands 'gralph help worktree commands' commands "$@"
}
(( $+functions[_gralph__help__worktree__create_commands] )) ||
_gralph__help__worktree__create_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help worktree create commands' commands "$@"
}
(( $+functions[_gralph__help__worktree__finish_commands] )) ||
_gralph__help__worktree__finish_commands() {
    local commands; commands=()
    _describe -t commands 'gralph help worktree finish commands' commands "$@"
}
(( $+functions[_gralph__init_commands] )) ||
_gralph__init_commands() {
    local commands; commands=()
    _describe -t commands 'gralph init commands' commands "$@"
}
(( $+functions[_gralph__logs_commands] )) ||
_gralph__logs_commands() {
    local commands; commands=()
    _describe -t commands 'gralph logs commands' commands "$@"
}
(( $+functions[_gralph__prd_commands] )) ||
_gralph__prd_commands() {
    local commands; commands=(
'check:Validate PRD task blocks' \
'create:Generate a spec-compliant PRD' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph prd commands' commands "$@"
}
(( $+functions[_gralph__prd__check_commands] )) ||
_gralph__prd__check_commands() {
    local commands; commands=()
    _describe -t commands 'gralph prd check commands' commands "$@"
}
(( $+functions[_gralph__prd__create_commands] )) ||
_gralph__prd__create_commands() {
    local commands; commands=()
    _describe -t commands 'gralph prd create commands' commands "$@"
}
(( $+functions[_gralph__prd__help_commands] )) ||
_gralph__prd__help_commands() {
    local commands; commands=(
'check:Validate PRD task blocks' \
'create:Generate a spec-compliant PRD' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph prd help commands' commands "$@"
}
(( $+functions[_gralph__prd__help__check_commands] )) ||
_gralph__prd__help__check_commands() {
    local commands; commands=()
    _describe -t commands 'gralph prd help check commands' commands "$@"
}
(( $+functions[_gralph__prd__help__create_commands] )) ||
_gralph__prd__help__create_commands() {
    local commands; commands=()
    _describe -t commands 'gralph prd help create commands' commands "$@"
}
(( $+functions[_gralph__prd__help__help_commands] )) ||
_gralph__prd__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'gralph prd help help commands' commands "$@"
}
(( $+functions[_gralph__resume_commands] )) ||
_gralph__resume_commands() {
    local commands; commands=()
    _describe -t commands 'gralph resume commands' commands "$@"
}
(( $+functions[_gralph__run-loop_commands] )) ||
_gralph__run-loop_commands() {
    local commands; commands=()
    _describe -t commands 'gralph run-loop commands' commands "$@"
}
(( $+functions[_gralph__server_commands] )) ||
_gralph__server_commands() {
    local commands; commands=()
    _describe -t commands 'gralph server commands' commands "$@"
}
(( $+functions[_gralph__start_commands] )) ||
_gralph__start_commands() {
    local commands; commands=()
    _describe -t commands 'gralph start commands' commands "$@"
}
(( $+functions[_gralph__status_commands] )) ||
_gralph__status_commands() {
    local commands; commands=()
    _describe -t commands 'gralph status commands' commands "$@"
}
(( $+functions[_gralph__stop_commands] )) ||
_gralph__stop_commands() {
    local commands; commands=()
    _describe -t commands 'gralph stop commands' commands "$@"
}
(( $+functions[_gralph__update_commands] )) ||
_gralph__update_commands() {
    local commands; commands=()
    _describe -t commands 'gralph update commands' commands "$@"
}
(( $+functions[_gralph__version_commands] )) ||
_gralph__version_commands() {
    local commands; commands=()
    _describe -t commands 'gralph version commands' commands "$@"
}
(( $+functions[_gralph__worktree_commands] )) ||
_gralph__worktree_commands() {
    local commands; commands=(
'create:Create task worktree' \
'finish:Finish task worktree' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph worktree commands' commands "$@"
}
(( $+functions[_gralph__worktree__create_commands] )) ||
_gralph__worktree__create_commands() {
    local commands; commands=()
    _describe -t commands 'gralph worktree create commands' commands "$@"
}
(( $+functions[_gralph__worktree__finish_commands] )) ||
_gralph__worktree__finish_commands() {
    local commands; commands=()
    _describe -t commands 'gralph worktree finish commands' commands "$@"
}
(( $+functions[_gralph__worktree__help_commands] )) ||
_gralph__worktree__help_commands() {
    local commands; commands=(
'create:Create task worktree' \
'finish:Finish task worktree' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'gralph worktree help commands' commands "$@"
}
(( $+functions[_gralph__worktree__help__create_commands] )) ||
_gralph__worktree__help__create_commands() {
    local commands; commands=()
    _describe -t commands 'gralph worktree help create commands' commands "$@"
}
(( $+functions[_gralph__worktree__help__finish_commands] )) ||
_gralph__worktree__help__finish_commands() {
    local commands; commands=()
    _describe -t commands 'gralph worktree help finish commands' commands "$@"
}
(( $+functions[_gralph__worktree__help__help_commands] )) ||
_gralph__worktree__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'gralph worktree help help commands' commands "$@"
}

if [ "$funcstack[1]" = "_gralph" ]; then
    _gralph "$@"
else
    compdef _gralph gralph
fi
