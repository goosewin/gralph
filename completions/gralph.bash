# bash completion for gralph                               -*- shell-script -*-

__gralph_debug()
{
    if [[ -n ${BASH_COMP_DEBUG_FILE:-} ]]; then
        echo "$*" >> "${BASH_COMP_DEBUG_FILE}"
    fi
}

# Homebrew on Macs have version 1.3 of bash-completion which doesn't include
# _init_completion. This is a very minimal version of that function.
__gralph_init_completion()
{
    COMPREPLY=()
    _get_comp_words_by_ref "$@" cur prev words cword
}

__gralph_index_of_word()
{
    local w word=$1
    shift
    index=0
    for w in "$@"; do
        [[ $w = "$word" ]] && return
        index=$((index+1))
    done
    index=-1
}

__gralph_contains_word()
{
    local w word=$1; shift
    for w in "$@"; do
        [[ $w = "$word" ]] && return
    done
    return 1
}

__gralph_handle_go_custom_completion()
{
    __gralph_debug "${FUNCNAME[0]}: cur is ${cur}, words[*] is ${words[*]}, #words[@] is ${#words[@]}"

    local shellCompDirectiveError=1
    local shellCompDirectiveNoSpace=2
    local shellCompDirectiveNoFileComp=4
    local shellCompDirectiveFilterFileExt=8
    local shellCompDirectiveFilterDirs=16

    local out requestComp lastParam lastChar comp directive args

    # Prepare the command to request completions for the program.
    # Calling ${words[0]} instead of directly gralph allows handling aliases
    args=("${words[@]:1}")
    # Disable ActiveHelp which is not supported for bash completion v1
    requestComp="GRALPH_ACTIVE_HELP=0 ${words[0]} __completeNoDesc ${args[*]}"

    lastParam=${words[$((${#words[@]}-1))]}
    lastChar=${lastParam:$((${#lastParam}-1)):1}
    __gralph_debug "${FUNCNAME[0]}: lastParam ${lastParam}, lastChar ${lastChar}"

    if [ -z "${cur}" ] && [ "${lastChar}" != "=" ]; then
        # If the last parameter is complete (there is a space following it)
        # We add an extra empty parameter so we can indicate this to the go method.
        __gralph_debug "${FUNCNAME[0]}: Adding extra empty parameter"
        requestComp="${requestComp} \"\""
    fi

    __gralph_debug "${FUNCNAME[0]}: calling ${requestComp}"
    # Use eval to handle any environment variables and such
    out=$(eval "${requestComp}" 2>/dev/null)

    # Extract the directive integer at the very end of the output following a colon (:)
    directive=${out##*:}
    # Remove the directive
    out=${out%:*}
    if [ "${directive}" = "${out}" ]; then
        # There is not directive specified
        directive=0
    fi
    __gralph_debug "${FUNCNAME[0]}: the completion directive is: ${directive}"
    __gralph_debug "${FUNCNAME[0]}: the completions are: ${out}"

    if [ $((directive & shellCompDirectiveError)) -ne 0 ]; then
        # Error code.  No completion.
        __gralph_debug "${FUNCNAME[0]}: received error from custom completion go code"
        return
    else
        if [ $((directive & shellCompDirectiveNoSpace)) -ne 0 ]; then
            if [[ $(type -t compopt) = "builtin" ]]; then
                __gralph_debug "${FUNCNAME[0]}: activating no space"
                compopt -o nospace
            fi
        fi
        if [ $((directive & shellCompDirectiveNoFileComp)) -ne 0 ]; then
            if [[ $(type -t compopt) = "builtin" ]]; then
                __gralph_debug "${FUNCNAME[0]}: activating no file completion"
                compopt +o default
            fi
        fi
    fi

    if [ $((directive & shellCompDirectiveFilterFileExt)) -ne 0 ]; then
        # File extension filtering
        local fullFilter filter filteringCmd
        # Do not use quotes around the $out variable or else newline
        # characters will be kept.
        for filter in ${out}; do
            fullFilter+="$filter|"
        done

        filteringCmd="_filedir $fullFilter"
        __gralph_debug "File filtering command: $filteringCmd"
        $filteringCmd
    elif [ $((directive & shellCompDirectiveFilterDirs)) -ne 0 ]; then
        # File completion for directories only
        local subdir
        # Use printf to strip any trailing newline
        subdir=$(printf "%s" "${out}")
        if [ -n "$subdir" ]; then
            __gralph_debug "Listing directories in $subdir"
            __gralph_handle_subdirs_in_dir_flag "$subdir"
        else
            __gralph_debug "Listing directories in ."
            _filedir -d
        fi
    else
        while IFS='' read -r comp; do
            COMPREPLY+=("$comp")
        done < <(compgen -W "${out}" -- "$cur")
    fi
}

__gralph_handle_reply()
{
    __gralph_debug "${FUNCNAME[0]}"
    local comp
    case $cur in
        -* )
            if [[ $(type -t compopt) = "builtin" ]]; then
                compopt -o nospace
            fi
            local allflags
            if [ ${#must_have_one_flag[@]} -ne 0 ]; then
                allflags=("${must_have_one_flag[@]}")
            else
                allflags=("${flags[*]} ${two_word_flags[*]}")
            fi
            while IFS='' read -r comp; do
                COMPREPLY+=("$comp")
            done < <(compgen -W "${allflags[*]}" -- "$cur")
            if [[ $(type -t compopt) = "builtin" ]]; then
                [[ "${COMPREPLY[0]}" == *= ]] || compopt +o nospace
            fi

            # complete after --flag=abc
            if [[ $cur == *=* ]]; then
                if [[ $(type -t compopt) = "builtin" ]]; then
                    compopt +o nospace
                fi

                local index flag
                flag="${cur%=*}"
                __gralph_index_of_word "${flag}" "${flags_with_completion[@]}"
                COMPREPLY=()
                if [[ ${index} -ge 0 ]]; then
                    PREFIX=""
                    cur="${cur#*=}"
                    ${flags_completion[${index}]}
                    if [ -n "${ZSH_VERSION:-}" ]; then
                        # zsh completion needs --flag= prefix
                        eval "COMPREPLY=( \"\${COMPREPLY[@]/#/${flag}=}\" )"
                    fi
                fi
            fi

            if [[ -z "${flag_parsing_disabled}" ]]; then
                # If flag parsing is enabled, we have completed the flags and can return.
                # If flag parsing is disabled, we may not know all (or any) of the flags, so we fallthrough
                # to possibly call handle_go_custom_completion.
                return 0;
            fi
            ;;
    esac

    # check if we are handling a flag with special work handling
    local index
    __gralph_index_of_word "${prev}" "${flags_with_completion[@]}"
    if [[ ${index} -ge 0 ]]; then
        ${flags_completion[${index}]}
        return
    fi

    # we are parsing a flag and don't have a special handler, no completion
    if [[ ${cur} != "${words[cword]}" ]]; then
        return
    fi

    local completions
    completions=("${commands[@]}")
    if [[ ${#must_have_one_noun[@]} -ne 0 ]]; then
        completions+=("${must_have_one_noun[@]}")
    elif [[ -n "${has_completion_function}" ]]; then
        # if a go completion function is provided, defer to that function
        __gralph_handle_go_custom_completion
    fi
    if [[ ${#must_have_one_flag[@]} -ne 0 ]]; then
        completions+=("${must_have_one_flag[@]}")
    fi
    while IFS='' read -r comp; do
        COMPREPLY+=("$comp")
    done < <(compgen -W "${completions[*]}" -- "$cur")

    if [[ ${#COMPREPLY[@]} -eq 0 && ${#noun_aliases[@]} -gt 0 && ${#must_have_one_noun[@]} -ne 0 ]]; then
        while IFS='' read -r comp; do
            COMPREPLY+=("$comp")
        done < <(compgen -W "${noun_aliases[*]}" -- "$cur")
    fi

    if [[ ${#COMPREPLY[@]} -eq 0 ]]; then
        if declare -F __gralph_custom_func >/dev/null; then
            # try command name qualified custom func
            __gralph_custom_func
        else
            # otherwise fall back to unqualified for compatibility
            declare -F __custom_func >/dev/null && __custom_func
        fi
    fi

    # available in bash-completion >= 2, not always present on macOS
    if declare -F __ltrim_colon_completions >/dev/null; then
        __ltrim_colon_completions "$cur"
    fi

    # If there is only 1 completion and it is a flag with an = it will be completed
    # but we don't want a space after the =
    if [[ "${#COMPREPLY[@]}" -eq "1" ]] && [[ $(type -t compopt) = "builtin" ]] && [[ "${COMPREPLY[0]}" == --*= ]]; then
       compopt -o nospace
    fi
}

# The arguments should be in the form "ext1|ext2|extn"
__gralph_handle_filename_extension_flag()
{
    local ext="$1"
    _filedir "@(${ext})"
}

__gralph_handle_subdirs_in_dir_flag()
{
    local dir="$1"
    pushd "${dir}" >/dev/null 2>&1 && _filedir -d && popd >/dev/null 2>&1 || return
}

__gralph_handle_flag()
{
    __gralph_debug "${FUNCNAME[0]}: c is $c words[c] is ${words[c]}"

    # if a command required a flag, and we found it, unset must_have_one_flag()
    local flagname=${words[c]}
    local flagvalue=""
    # if the word contained an =
    if [[ ${words[c]} == *"="* ]]; then
        flagvalue=${flagname#*=} # take in as flagvalue after the =
        flagname=${flagname%=*} # strip everything after the =
        flagname="${flagname}=" # but put the = back
    fi
    __gralph_debug "${FUNCNAME[0]}: looking for ${flagname}"
    if __gralph_contains_word "${flagname}" "${must_have_one_flag[@]}"; then
        must_have_one_flag=()
    fi

    # if you set a flag which only applies to this command, don't show subcommands
    if __gralph_contains_word "${flagname}" "${local_nonpersistent_flags[@]}"; then
      commands=()
    fi

    # keep flag value with flagname as flaghash
    # flaghash variable is an associative array which is only supported in bash > 3.
    if [[ -z "${BASH_VERSION:-}" || "${BASH_VERSINFO[0]:-}" -gt 3 ]]; then
        if [ -n "${flagvalue}" ] ; then
            flaghash[${flagname}]=${flagvalue}
        elif [ -n "${words[ $((c+1)) ]}" ] ; then
            flaghash[${flagname}]=${words[ $((c+1)) ]}
        else
            flaghash[${flagname}]="true" # pad "true" for bool flag
        fi
    fi

    # skip the argument to a two word flag
    if [[ ${words[c]} != *"="* ]] && __gralph_contains_word "${words[c]}" "${two_word_flags[@]}"; then
        __gralph_debug "${FUNCNAME[0]}: found a flag ${words[c]}, skip the next argument"
        c=$((c+1))
        # if we are looking for a flags value, don't show commands
        if [[ $c -eq $cword ]]; then
            commands=()
        fi
    fi

    c=$((c+1))

}

__gralph_handle_noun()
{
    __gralph_debug "${FUNCNAME[0]}: c is $c words[c] is ${words[c]}"

    if __gralph_contains_word "${words[c]}" "${must_have_one_noun[@]}"; then
        must_have_one_noun=()
    elif __gralph_contains_word "${words[c]}" "${noun_aliases[@]}"; then
        must_have_one_noun=()
    fi

    nouns+=("${words[c]}")
    c=$((c+1))
}

__gralph_handle_command()
{
    __gralph_debug "${FUNCNAME[0]}: c is $c words[c] is ${words[c]}"

    local next_command
    if [[ -n ${last_command} ]]; then
        next_command="_${last_command}_${words[c]//:/__}"
    else
        if [[ $c -eq 0 ]]; then
            next_command="_gralph_root_command"
        else
            next_command="_${words[c]//:/__}"
        fi
    fi
    c=$((c+1))
    __gralph_debug "${FUNCNAME[0]}: looking for ${next_command}"
    declare -F "$next_command" >/dev/null && $next_command
}

__gralph_handle_word()
{
    if [[ $c -ge $cword ]]; then
        __gralph_handle_reply
        return
    fi
    __gralph_debug "${FUNCNAME[0]}: c is $c words[c] is ${words[c]}"
    if [[ "${words[c]}" == -* ]]; then
        __gralph_handle_flag
    elif __gralph_contains_word "${words[c]}" "${commands[@]}"; then
        __gralph_handle_command
    elif [[ $c -eq 0 ]]; then
        __gralph_handle_command
    elif __gralph_contains_word "${words[c]}" "${command_aliases[@]}"; then
        # aliashash variable is an associative array which is only supported in bash > 3.
        if [[ -z "${BASH_VERSION:-}" || "${BASH_VERSINFO[0]:-}" -gt 3 ]]; then
            words[c]=${aliashash[${words[c]}]}
            __gralph_handle_command
        else
            __gralph_handle_noun
        fi
    else
        __gralph_handle_noun
    fi
    __gralph_handle_word
}

_gralph_backends()
{
    last_command="gralph_backends"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_completion()
{
    last_command="gralph_completion"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--help")
    flags+=("-h")
    local_nonpersistent_flags+=("--help")
    local_nonpersistent_flags+=("-h")

    must_have_one_flag=()
    must_have_one_noun=()
    must_have_one_noun+=("bash")
    must_have_one_noun+=("fish")
    must_have_one_noun+=("zsh")
    noun_aliases=()
}

_gralph_config_get()
{
    last_command="gralph_config_get"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_config_list()
{
    last_command="gralph_config_list"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_config_set()
{
    last_command="gralph_config_set"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_config()
{
    last_command="gralph_config"

    command_aliases=()

    commands=()
    commands+=("get")
    commands+=("list")
    commands+=("set")

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_help()
{
    last_command="gralph_help"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    has_completion_function=1
    noun_aliases=()
}

_gralph_logs()
{
    last_command="gralph_logs"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--follow")
    local_nonpersistent_flags+=("--follow")

    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_prd_check()
{
    last_command="gralph_prd_check"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--allow-missing-context")
    local_nonpersistent_flags+=("--allow-missing-context")

    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_prd_create()
{
    last_command="gralph_prd_create"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--allow-missing-context")
    local_nonpersistent_flags+=("--allow-missing-context")
    flags+=("--backend=")
    two_word_flags+=("--backend")
    two_word_flags+=("-b")
    local_nonpersistent_flags+=("--backend")
    local_nonpersistent_flags+=("--backend=")
    local_nonpersistent_flags+=("-b")
    flags+=("--constraints=")
    two_word_flags+=("--constraints")
    local_nonpersistent_flags+=("--constraints")
    local_nonpersistent_flags+=("--constraints=")
    flags+=("--context=")
    two_word_flags+=("--context")
    local_nonpersistent_flags+=("--context")
    local_nonpersistent_flags+=("--context=")
    flags+=("--dir=")
    two_word_flags+=("--dir")
    local_nonpersistent_flags+=("--dir")
    local_nonpersistent_flags+=("--dir=")
    flags+=("--force")
    local_nonpersistent_flags+=("--force")
    flags+=("--goal=")
    two_word_flags+=("--goal")
    local_nonpersistent_flags+=("--goal")
    local_nonpersistent_flags+=("--goal=")
    flags+=("--interactive")
    local_nonpersistent_flags+=("--interactive")
    flags+=("--model=")
    two_word_flags+=("--model")
    two_word_flags+=("-m")
    local_nonpersistent_flags+=("--model")
    local_nonpersistent_flags+=("--model=")
    local_nonpersistent_flags+=("-m")
    flags+=("--multiline")
    local_nonpersistent_flags+=("--multiline")
    flags+=("--no-interactive")
    local_nonpersistent_flags+=("--no-interactive")
    flags+=("--output=")
    two_word_flags+=("--output")
    two_word_flags+=("-o")
    local_nonpersistent_flags+=("--output")
    local_nonpersistent_flags+=("--output=")
    local_nonpersistent_flags+=("-o")
    flags+=("--sources=")
    two_word_flags+=("--sources")
    local_nonpersistent_flags+=("--sources")
    local_nonpersistent_flags+=("--sources=")

    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_prd()
{
    last_command="gralph_prd"

    command_aliases=()

    commands=()
    commands+=("check")
    commands+=("create")
    if [[ -z "${BASH_VERSION:-}" || "${BASH_VERSINFO[0]:-}" -gt 3 ]]; then
        command_aliases+=("init")
        aliashash["init"]="create"
        command_aliases+=("new")
        aliashash["new"]="create"
    fi

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_resume()
{
    last_command="gralph_resume"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_server()
{
    last_command="gralph_server"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--host=")
    two_word_flags+=("--host")
    two_word_flags+=("-H")
    local_nonpersistent_flags+=("--host")
    local_nonpersistent_flags+=("--host=")
    local_nonpersistent_flags+=("-H")
    flags+=("--open")
    local_nonpersistent_flags+=("--open")
    flags+=("--port=")
    two_word_flags+=("--port")
    two_word_flags+=("-p")
    local_nonpersistent_flags+=("--port")
    local_nonpersistent_flags+=("--port=")
    local_nonpersistent_flags+=("-p")
    flags+=("--token=")
    two_word_flags+=("--token")
    two_word_flags+=("-t")
    local_nonpersistent_flags+=("--token")
    local_nonpersistent_flags+=("--token=")
    local_nonpersistent_flags+=("-t")

    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_start()
{
    last_command="gralph_start"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--backend=")
    two_word_flags+=("--backend")
    two_word_flags+=("-b")
    local_nonpersistent_flags+=("--backend")
    local_nonpersistent_flags+=("--backend=")
    local_nonpersistent_flags+=("-b")
    flags+=("--completion-marker=")
    two_word_flags+=("--completion-marker")
    local_nonpersistent_flags+=("--completion-marker")
    local_nonpersistent_flags+=("--completion-marker=")
    flags+=("--max-iterations=")
    two_word_flags+=("--max-iterations")
    local_nonpersistent_flags+=("--max-iterations")
    local_nonpersistent_flags+=("--max-iterations=")
    flags+=("--model=")
    two_word_flags+=("--model")
    two_word_flags+=("-m")
    local_nonpersistent_flags+=("--model")
    local_nonpersistent_flags+=("--model=")
    local_nonpersistent_flags+=("-m")
    flags+=("--name=")
    two_word_flags+=("--name")
    two_word_flags+=("-n")
    local_nonpersistent_flags+=("--name")
    local_nonpersistent_flags+=("--name=")
    local_nonpersistent_flags+=("-n")
    flags+=("--no-tmux")
    local_nonpersistent_flags+=("--no-tmux")
    flags+=("--prompt-template=")
    two_word_flags+=("--prompt-template")
    local_nonpersistent_flags+=("--prompt-template")
    local_nonpersistent_flags+=("--prompt-template=")
    flags+=("--strict-prd")
    local_nonpersistent_flags+=("--strict-prd")
    flags+=("--task-file=")
    two_word_flags+=("--task-file")
    two_word_flags+=("-f")
    local_nonpersistent_flags+=("--task-file")
    local_nonpersistent_flags+=("--task-file=")
    local_nonpersistent_flags+=("-f")
    flags+=("--variant=")
    two_word_flags+=("--variant")
    local_nonpersistent_flags+=("--variant")
    local_nonpersistent_flags+=("--variant=")
    flags+=("--webhook=")
    two_word_flags+=("--webhook")
    local_nonpersistent_flags+=("--webhook")
    local_nonpersistent_flags+=("--webhook=")

    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_status()
{
    last_command="gralph_status"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_stop()
{
    last_command="gralph_stop"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()

    flags+=("--all")
    flags+=("-a")
    local_nonpersistent_flags+=("--all")
    local_nonpersistent_flags+=("-a")

    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_worktree_create()
{
    last_command="gralph_worktree_create"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_worktree_finish()
{
    last_command="gralph_worktree_finish"

    command_aliases=()

    commands=()

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_worktree()
{
    last_command="gralph_worktree"

    command_aliases=()

    commands=()
    commands+=("create")
    commands+=("finish")

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

_gralph_root_command()
{
    last_command="gralph"

    command_aliases=()

    commands=()
    commands+=("backends")
    commands+=("completion")
    commands+=("config")
    commands+=("help")
    commands+=("logs")
    commands+=("prd")
    commands+=("resume")
    commands+=("server")
    commands+=("start")
    commands+=("status")
    commands+=("stop")
    commands+=("worktree")

    flags=()
    two_word_flags=()
    local_nonpersistent_flags=()
    flags_with_completion=()
    flags_completion=()


    must_have_one_flag=()
    must_have_one_noun=()
    noun_aliases=()
}

__start_gralph()
{
    local cur prev words cword split
    declare -A flaghash 2>/dev/null || :
    declare -A aliashash 2>/dev/null || :
    if declare -F _init_completion >/dev/null 2>&1; then
        _init_completion -s || return
    else
        __gralph_init_completion -n "=" || return
    fi

    local c=0
    local flag_parsing_disabled=
    local flags=()
    local two_word_flags=()
    local local_nonpersistent_flags=()
    local flags_with_completion=()
    local flags_completion=()
    local commands=("gralph")
    local command_aliases=()
    local must_have_one_flag=()
    local must_have_one_noun=()
    local has_completion_function=""
    local last_command=""
    local nouns=()
    local noun_aliases=()

    __gralph_handle_word
}

if [[ $(type -t compopt) = "builtin" ]]; then
    complete -o default -F __start_gralph gralph
else
    complete -o default -o nospace -F __start_gralph gralph
fi

# ex: ts=4 sw=4 et filetype=sh
