#compdef gralph
#
# Zsh completions for gralph
#
# Installation:
#   - Copy to a directory in your $fpath (e.g., ~/.zsh/completions/)
#   - Or add to /usr/local/share/zsh/site-functions/_gralph
#   - Ensure 'compinit' is called in your .zshrc

_gralph() {
    local -a commands
    local -a start_opts stop_opts logs_opts server_opts

    commands=(
        'start:Start a new gralph loop'
        'stop:Stop a running loop'
        'status:Show status of all loops'
        'logs:View logs for a loop'
        'resume:Resume crashed/stopped loops'
        'backends:List available AI backends'
        'config:Manage configuration'
        'server:Start status API server'
        'version:Show version'
        'help:Show help message'
    )

    start_opts=(
        '(-n --name)'{-n,--name}'[Session name]:name:'
        '--max-iterations[Max iterations before giving up]:iterations:(10 20 30 50 100)'
        '(-f --task-file)'{-f,--task-file}'[Task file path]:file:_files -g "*.md"'
        '--completion-marker[Completion promise text]:marker:(COMPLETE DONE FINISHED ALL_DONE)'
        '(-b --backend)'{-b,--backend}'[AI backend to use]:backend:(claude opencode)'
        '(-m --model)'{-m,--model}'[Model override]:model:(claude-sonnet-4-20250514 claude-opus-4-20250514 claude-haiku-3-5-20241022 anthropic/claude-sonnet-4-20250514 anthropic/claude-opus-4-20250514 openai/gpt-4o google/gemini-2.0-flash deepseek/deepseek-chat)'
        '--webhook[Notification webhook URL]:url:'
        '--no-tmux[Run in foreground (blocks)]'
        '(-h --help)'{-h,--help}'[Show help]'
    )

    stop_opts=(
        '(-a --all)'{-a,--all}'[Stop all loops]'
        '(-h --help)'{-h,--help}'[Show help]'
    )

    logs_opts=(
        '--follow[Follow log output continuously]'
        '(-h --help)'{-h,--help}'[Show help]'
    )

    server_opts=(
        '(-p --port)'{-p,--port}'[Port number]:port:(8080 3000 8000 9000)'
        '(-t --token)'{-t,--token}'[Authentication token]:token:'
        '(-h --help)'{-h,--help}'[Show help]'
    )

    _arguments -C \
        '1: :->command' \
        '*:: :->args'

    case $state in
        command)
            _describe -t commands 'gralph commands' commands
            ;;
        args)
            case $words[1] in
                start)
                    _arguments $start_opts \
                        '1:directory:_directories'
                    ;;
                stop)
                    _arguments $stop_opts \
                        '1:session:_gralph_sessions'
                    ;;
                logs)
                    _arguments $logs_opts \
                        '1:session:_gralph_sessions'
                    ;;
                resume)
                    _arguments \
                        '1:session:_gralph_sessions'
                    ;;
                server)
                    _arguments $server_opts
                    ;;
                config)
                    local -a config_cmds
                    config_cmds=(
                        'get:Get configuration value'
                        'set:Set configuration value'
                        'list:List all configuration'
                    )
                    _describe -t config_cmds 'config subcommands' config_cmds
                    ;;
                backends|status|version|help)
                    # No further arguments
                    ;;
            esac
            ;;
    esac
}

# Helper function to get session names
_gralph_sessions() {
    local -a sessions
    local state_file="${HOME}/.config/gralph/state.json"

    if [[ -f "$state_file" ]] && (( $+commands[jq] )); then
        sessions=(${(f)"$(jq -r '.sessions | keys[]' "$state_file" 2>/dev/null)"})
        if [[ -n "$sessions" ]]; then
            _describe -t sessions 'gralph sessions' sessions
            return
        fi
    fi

    _message 'no sessions found'
}

_gralph "$@"
