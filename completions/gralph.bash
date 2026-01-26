# Bash completions for gralph
#
# Installation:
#   - Copy to /etc/bash_completion.d/gralph  (system-wide)
#   - Or add to ~/.bashrc: source /path/to/gralph.bash
#   - Or copy to ~/.local/share/bash-completion/completions/gralph

_gralph_completions() {
    local cur prev words cword
    _init_completion 2>/dev/null || {
        COMPREPLY=()
        cur="${COMP_WORDS[COMP_CWORD]}"
        prev="${COMP_WORDS[COMP_CWORD-1]}"
        words=("${COMP_WORDS[@]}")
        cword=$COMP_CWORD
    }

    # Main commands
    local commands="start stop status logs resume prd backends config server version help"

    # Options for start command
    local start_opts="--name -n --max-iterations --task-file -f --completion-marker --backend -b --model -m --variant --webhook --no-tmux --help -h"

    # Options for stop command
    local stop_opts="--all -a --help -h"

    # Options for logs command
    local logs_opts="--follow --help -h"

    # Options for server command
    local server_opts="--host -H --port -p --token -t --open --help -h"

    # Determine the command (first non-option argument after 'gralph')
    local cmd=""
    local i
    for ((i=1; i < cword; i++)); do
        case "${words[i]}" in
            -*)
                continue
                ;;
            start|stop|status|logs|resume|prd|config|server|version|help)
                cmd="${words[i]}"
                break
                ;;
        esac
    done

    # If no command yet, complete commands
    if [[ -z "$cmd" ]]; then
        COMPREPLY=($(compgen -W "$commands" -- "$cur"))
        return 0
    fi

    # Command-specific completions
    case "$cmd" in
        start)
            case "$prev" in
                -n|--name)
                    # Session name - no specific completion
                    return 0
                    ;;
                --max-iterations)
                    # Suggest common iteration counts
                    COMPREPLY=($(compgen -W "10 20 30 50 100" -- "$cur"))
                    return 0
                    ;;
                -f|--task-file)
                    # Complete with markdown files
                    COMPREPLY=($(compgen -f -X '!*.md' -- "$cur"))
                    return 0
                    ;;
                --completion-marker)
                    # Suggest common markers
                    COMPREPLY=($(compgen -W "COMPLETE DONE FINISHED ALL_DONE" -- "$cur"))
                    return 0
                    ;;
                -b|--backend)
                    # Suggest available backends
                    COMPREPLY=($(compgen -W "claude opencode gemini codex" -- "$cur"))
                    return 0
                    ;;
                -m|--model)
                    # Suggest models for all backends
                    # Claude models
                    local claude_models="claude-opus-4-5"
                    # OpenCode models (provider/model format)
                    local opencode_models="opencode/example-code-model anthropic/claude-opus-4-5 google/gemini-1.5-pro"
                    # Gemini models (native backend)
                    local gemini_models="gemini-1.5-pro"
                    # Codex models (native backend)
                    local codex_models="example-codex-model"
                    COMPREPLY=($(compgen -W "$claude_models $opencode_models $gemini_models $codex_models" -- "$cur"))
                    return 0
                    ;;
                --variant)
                    # Suggest common variants
                    COMPREPLY=($(compgen -W "xhigh high medium low" -- "$cur"))
                    return 0
                    ;;
                --webhook)
                    # URL - no specific completion
                    return 0
                    ;;
            esac

            # If current word starts with -, complete options
            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "$start_opts" -- "$cur"))
                return 0
            fi

            # Otherwise complete directories
            COMPREPLY=($(compgen -d -- "$cur"))
            return 0
            ;;

        stop)
            case "$prev" in
                stop)
                    # Complete with session names or --all
                    local sessions=""
                    if command -v gralph &>/dev/null; then
                        sessions=$(_gralph_get_sessions)
                    fi
                    COMPREPLY=($(compgen -W "$sessions --all -a" -- "$cur"))
                    return 0
                    ;;
            esac

            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "$stop_opts" -- "$cur"))
                return 0
            fi

            # Complete with session names
            local sessions=""
            if command -v gralph &>/dev/null; then
                sessions=$(_gralph_get_sessions)
            fi
            COMPREPLY=($(compgen -W "$sessions" -- "$cur"))
            return 0
            ;;

        logs)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "$logs_opts" -- "$cur"))
                return 0
            fi

            # Complete with session names
            local sessions=""
            if command -v gralph &>/dev/null; then
                sessions=$(_gralph_get_sessions)
            fi
            COMPREPLY=($(compgen -W "$sessions" -- "$cur"))
            return 0
            ;;

        prd)
            case "$prev" in
                prd)
                    COMPREPLY=($(compgen -W "check create" -- "$cur"))
                    return 0
                    ;;
                check)
                    COMPREPLY=($(compgen -f -X '!*.md' -- "$cur"))
                    return 0
                    ;;
            esac

            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "--dir --output -o --goal --constraints --context --sources --allow-missing-context --multiline --no-interactive --interactive --force" -- "$cur"))
                return 0
            fi

            return 0
            ;;

        resume)
            # Complete with session names
            local sessions=""
            if command -v gralph &>/dev/null; then
                sessions=$(_gralph_get_sessions)
            fi
            COMPREPLY=($(compgen -W "$sessions" -- "$cur"))
            return 0
            ;;

        server)
            case "$prev" in
                -H|--host)
                    # Suggest common host bindings
                    COMPREPLY=($(compgen -W "127.0.0.1 0.0.0.0 localhost" -- "$cur"))
                    return 0
                    ;;
                -p|--port)
                    # Suggest common ports
                    COMPREPLY=($(compgen -W "8080 3000 8000 9000" -- "$cur"))
                    return 0
                    ;;
                -t|--token)
                    # Token - no specific completion
                    return 0
                    ;;
            esac

            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "$server_opts" -- "$cur"))
                return 0
            fi
            return 0
            ;;

        config)
            # Config subcommands
            COMPREPLY=($(compgen -W "get set list" -- "$cur"))
            return 0
            ;;

        status|version|help)
            # No further completions needed
            return 0
            ;;
    esac

    return 0
}

# Helper function to get session names from state file
_gralph_get_sessions() {
    local state_file="${HOME}/.config/gralph/state.json"
    if [[ -f "$state_file" ]] && command -v jq &>/dev/null; then
        jq -r '.sessions | keys[]' "$state_file" 2>/dev/null
    fi
}

# Register the completion function
complete -F _gralph_completions gralph
