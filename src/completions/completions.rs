/// Generate a bash shell completion script for `smol`.
pub fn generate_bash() -> String {
    r###"_smol_completions() {
    local cur prev words cword
    _init_completion || return

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=($(compgen -W "status log list cancel clean benchmark completion help migrate" -- "$cur"))
        return
    fi

    case "${words[1]}" in
        status|log|cancel)
            # Complete task IDs
            local tasks_dir="${HOME}/.smol/tasks"
            if [[ -d "$tasks_dir" ]]; then
                local task_ids=""
                for task_dir in "$tasks_dir"/*/; do
                    task_ids+=" $(basename "$task_dir")"
                done
                COMPREPLY=($(compgen -W "$task_ids" -- "$cur"))
            fi
            ;;
        clean)
            COMPREPLY=($(compgen -W "--older" -- "$cur"))
            ;;
    esac
}

complete -F _smol_completions smol
"###
    .to_string()
}

/// Generate a zsh shell completion script for `smol`.
pub fn generate_zsh() -> String {
    r###"#compdef smol
_smol() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    _arguments \
        '1: :->command' \
        '*: :->args'

    case $state in
        command)
            _values 'command' \
                'status[Show task status]' \
                'log[Show task log]' \
                'list[List tasks]' \
                'cancel[Cancel a task]' \
                'clean[Clean old tasks]' \
                'benchmark[Run benchmarks]' \
                'completion[Generate completion]' \
                'help[Show help]' \
                'migrate[Migrate data]'
            ;;
        args)
            case $words[1] in
                status|log|cancel)
                    local tasks_dir="${HOME}/.smol/tasks"
                    if [[ -d "$tasks_dir" ]]; then
                        local ids=(${tasks_dir}/*(/:t))
                        _values 'task-id' $ids
                    fi
                    ;;
            esac
            ;;
    esac
}

_smol "$@"
"###
    .to_string()
}

/// Generate a fish shell completion script for `smol`.
pub fn generate_fish() -> String {
    r###"function __fish_smol_needs_command
    set cmd (commandline -opc)
    if [ (count $cmd) -eq 1 ]
        return 0
    end
    return 1
end

function __fish_smol_using_command
    set cmd (commandline -opc)
    if [ (count $cmd) -gt 1 ]
        if [ $argv[1] = $cmd[2] ]
            return 0
        end
    end
    return 1
end

function __fish_smol_print_task_ids
    set tasks_dir "$HOME/.smol/tasks"
    if test -d "$tasks_dir"
        for task_dir in "$tasks_dir"/*/
            echo (basename "$task_dir")
        end
    end
end

complete -f -c smol -n '__fish_smol_needs_command' -a status -d 'Show task status'
complete -f -c smol -n '__fish_smol_needs_command' -a log -d 'Show task log'
complete -f -c smol -n '__fish_smol_needs_command' -a list -d 'List tasks'
complete -f -c smol -n '__fish_smol_needs_command' -a cancel -d 'Cancel a task'
complete -f -c smol -n '__fish_smol_needs_command' -a clean -d 'Clean old tasks'
complete -f -c smol -n '__fish_smol_needs_command' -a benchmark -d 'Run benchmarks'
complete -f -c smol -n '__fish_smol_needs_command' -a completion -d 'Generate completion scripts'
complete -f -c smol -n '__fish_smol_needs_command' -a help -d 'Show help'
complete -f -c smol -n '__fish_smol_needs_command' -a migrate -d 'Migrate data'

# Task ID completion
complete -f -c smol -n '__fish_smol_using_command status' -a '(__fish_smol_print_task_ids)'
complete -f -c smol -n '__fish_smol_using_command log' -a '(__fish_smol_print_task_ids)'
complete -f -c smol -n '__fish_smol_using_command cancel' -a '(__fish_smol_print_task_ids)'
"###
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bash_contains_completion() {
        let script = generate_bash();
        assert!(script.contains("_smol_completions"));
        assert!(script.contains("status log list cancel clean benchmark completion help migrate"));
        assert!(script.contains("_init_completion"));
    }

    #[test]
    fn test_generate_zsh_contains_completion() {
        let script = generate_zsh();
        assert!(script.contains("#compdef smol"));
        assert!(script.contains("_smol"));
        assert!(script.contains("Show task status"));
    }

    #[test]
    fn test_generate_fish_contains_completion() {
        let script = generate_fish();
        assert!(script.contains("__fish_smol_needs_command"));
        assert!(script.contains("__fish_smol_print_task_ids"));
        assert!(script.contains("complete -f -c smol"));
    }
}
