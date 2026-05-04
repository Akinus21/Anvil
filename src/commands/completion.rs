use std::path::Path;
use std::fs;

pub fn execute(config_dir: &Path, args: Vec<String>) -> i32 {
    let shell = args.first().map(|s| s.as_str()).unwrap_or("bash");
    let install = args.iter().any(|a| a == "--install");

    match shell {
        "bash" => {
            let script = generate_bash_completion();
            if install {
                install_completion(config_dir, "bash", &script)
            } else {
                println!("{}", script);
                0
            }
        }
        "zsh" => {
            let script = generate_zsh_completion();
            if install {
                install_completion(config_dir, "zsh", &script)
            } else {
                println!("{}", script);
                0
            }
        }
        "fish" => {
            let script = generate_fish_completion();
            if install {
                install_completion(config_dir, "fish", &script)
            } else {
                println!("{}", script);
                0
            }
        }
        _ => {
            println!("Supported shells: bash, zsh, fish");
            println!("Usage: aktools completion <shell> [--install]");
            1
        }
    }
}

fn generate_bash_completion() -> String {
    let commands = "run add edit rm list update doctor help build-command edit_aliases completion add-repo list-repos search-mods install-mods add-mod update-mod inspect-mod autoupdate upgrade";
    format!(r#"# aktools bash completion

_aktools() {{
    local cur prev opts
    COMPREPLY=()
    cur="{{{{COMP_WORDS[COMP_CWORD]}}}}"
    prev="{{{{COMP_WORDS[COMP_CWORD-1]}}}}"
    opts="{}"

    case "${{prev}}" in
        run|edit|rm|inspect-mod)
            local modules=$(aktools list 2>/dev/null | grep -oE '^\S+' | tr '\n' ' ')
            COMPREPLY=($(compgen -W "${{modules}}" -- "${{cur}}"))
            ;;
        upgrade)
            COMPREPLY=($(compgen -W "aktools modules all" -- "${{cur}}"))
            ;;
        *)
            COMPREPLY=($(compgen -W "${{opts}}" -- "${{cur}}"))
            ;;
    esac
}}
complete -F _aktools aktools
"#, commands)
}

fn generate_zsh_completion() -> String {
    let commands_list = ["run", "add", "edit", "rm", "list", "update", "doctor", "help", "build-command", "edit_aliases", "completion", "add-repo", "list-repos", "search-mods", "search-mod", "mod-search", "install-mods", "install-mod", "mod-install", "add-mod", "update-mod", "inspect-mod", "autoupdate", "upgrade"];
    let commands = commands_list.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(" ");
    let upgrade_targets = "'aktools' 'modules' 'all'";
    format!(r#"# aktools zsh completion

_aktools() {{
    local -a commands modules upgrade_opts
    commands=({commands})
    upgrade_opts=({upgrade_targets})

    if (( CURRENT == 2 )); then
        _describe 'command' commands
        return
    fi

    case "${{words[2]}}" in
        run|edit|rm|inspect-mod)
            modules=($(aktools list 2>/dev/null | grep -oE '^\S+' | tr '\n' ' '))
            _describe 'module' modules
            ;;
        upgrade)
            _describe 'target' upgrade_opts
            ;;
    esac
}}

_aktools "$@"
"#)
}

fn generate_fish_completion() -> String {
    r#"# aktools fish completion

function __aktools_modules
    aktools list 2>/dev/null | grep -oE '^\S+' | tr '\n' ' '
end

complete -c aktools -n '__fish_seen_subcommand_from run edit rm inspect-mod' -a '(__aktools_modules)' -d 'module'
complete -c aktools -n '__fish_seen_subcommand_from upgrade' -a 'aktools modules all' -d 'target'
complete -c aktools -f -a 'run' -d 'Run a module'
complete -c aktools -f -a 'add' -d 'Add a module (-l for symlink)'
complete -c aktools -f -a 'edit' -d 'Edit a module manifest'
complete -c aktools -f -a 'rm' -d 'Remove a module'
complete -c aktools -f -a 'list' -d 'List installed modules'
complete -c aktools -f -a 'update' -d 'Rebuild the registry'
complete -c aktools -f -a 'doctor' -d 'Diagnose issues'
complete -c aktools -f -a 'help' -d 'Show help'
complete -c aktools -f -a 'build-command' -d 'Create command module'
complete -c aktools -f -a 'edit_aliases' -d 'Edit aliases'
complete -c aktools -f -a 'completion' -d 'Generate completions'
complete -c aktools -f -a 'add-repo' -d 'Add a repo'
complete -c aktools -f -a 'list-repos' -d 'List repos'
complete -c aktools -f -a 'search-mods' -d 'Search modules (alias: search-mod, mod-search)'
complete -c aktools -f -a 'install-mods' -d 'Install modules (alias: install-mod, mod-install)'
complete -c aktools -f -a 'add-mod' -d 'Submit module to repo'
complete -c aktools -f -a 'update-mod' -d 'Update module in repo (owner only)'
complete -c aktools -f -a 'inspect-mod' -d 'Show module contents'
complete -c aktools -f -a 'autoupdate' -d 'Manage autoupdate'
complete -c aktools -f -a 'upgrade' -d 'Upgrade aktools and/or modules'
"#.to_string()
}

fn install_completion(config_dir: &Path, shell: &str, script: &str) -> i32 {
    let completions_dir = config_dir.join("completions");
    if let Err(e) = fs::create_dir_all(&completions_dir) {
        println!("Error creating completions directory: {}", e);
        return 1;
    }

    let file_path = match shell {
        "bash" => completions_dir.join("aktools"),
        "zsh" => completions_dir.join("_aktools"),
        "fish" => completions_dir.join("aktools.fish"),
        _ => return 1,
    };

    if let Err(e) = fs::write(&file_path, script) {
        println!("Error writing completion file: {}", e);
        return 1;
    }

    let home = dirs::home_dir().unwrap_or_default();
    match shell {
        "bash" => {
            let rc_file = home.join(".bashrc");
            let completion_line = "source \"$AKTOOLS_HOME/completions/aktools\"\n";
            add_to_shell_config(&rc_file, "[ -f ~/.aktools/completions/aktools ] && source ~/.aktools/completions/aktools", completion_line);
        }
        "zsh" => {
            let rc_file = home.join(".zshrc");
            let completion_line = "fpath+=\"$AKTOOLS_HOME/completions\"\n";
            add_to_shell_config(&rc_file, "[ -f ~/.aktools/completions/_aktools ] && fpath+=~/.aktools/completions", &completion_line);
        }
        "fish" => {
            let config_dir_fish = home.join(".config/fish");
            if let Err(e) = fs::create_dir_all(&config_dir_fish) {
                println!("Error creating fish config directory: {}", e);
                return 1;
            }
            let completion_link = config_dir_fish.join("completions/aktools.fish");
            if let Some(parent) = completion_link.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::remove_file(&completion_link);
            if let Err(e) = std::os::unix::fs::symlink(&file_path, &completion_link) {
                println!("Note: Could not create symlink: {}", e);
            }
        }
        _ => return 1,
    }

    println!("Installed {} completion to {}", shell, file_path.display());
    0
}

fn add_to_shell_config(rc_file: &Path, check_pattern: &str, completion_line: &str) {
    if let Ok(content) = fs::read_to_string(rc_file) {
        if !content.contains(check_pattern) {
            let new_content = content.trim_end().to_string() + "\n" + completion_line;
            if let Err(e) = fs::write(rc_file, new_content) {
                println!("Error updating shell config: {}", e);
            }
        }
    }
}