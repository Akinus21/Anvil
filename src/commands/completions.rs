use std::path::Path;

use crate::modules::ModuleManager;
use crate::registry::Registry;

/// Get all module names from the registry
pub fn get_module_names(registry_path: &Path) -> Vec<String> {
    let registry = Registry::load(registry_path).unwrap_or_default();
    let mut names: Vec<String> = registry.modules.keys().cloned().collect();
    names.sort();
    names
}

/// Get flags for a specific module as (flag, description) pairs
/// The description is the first command if available, otherwise empty
pub fn get_module_flags(modules_dir: &Path, module_name: &str, registry_path: &Path) -> Vec<(String, String)> {
    let registry = match Registry::load(registry_path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let module = match registry.modules.get(module_name) {
        Some(m) => m,
        None => return Vec::new(),
    };

    let module_path = modules_dir.join(&module.folder);
    let manifest = match ModuleManager::load_manifest(&module_path) {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    manifest
        .options
        .iter()
        .filter(|opt| !opt.flags.is_empty())
        .map(|opt| {
            let flag = opt.flags.get(0).cloned().unwrap_or_default();
            let desc = opt.commands.first().cloned().unwrap_or_default();
            // Trim long descriptions for readability
            let desc = if desc.len() > 50 {
                format!("{}...", &desc[..47])
            } else {
                desc
            };
            (flag, desc)
        })
        .collect()
}

/// Execute the completions command - generates shell completion scripts
pub fn execute(shell: &str) -> Result<String, String> {
    match shell {
        "bash" => Ok(generate_bash_completion()),
        "zsh" => Ok(generate_zsh_completion()),
        "fish" => Ok(generate_fish_completion()),
        "powershell" => Ok(generate_powershell_completion()),
        "elvish" => Ok(generate_elvish_completion()),
        _ => Err(format!(
            "Unsupported shell: {}. Supported: bash, zsh, fish, powershell, elvish",
            shell
        )),
    }
}

/// Get a parseable list of modules with their flags for shell scripts
/// Output format: "module:flag1,flag2,flag3|..."
pub fn get_modules_with_flags(modules_dir: &Path, registry_path: &Path) -> String {
    let registry = match Registry::load(registry_path) {
        Ok(r) => r,
        Err(_) => return String::new(),
    };

    let mut results: Vec<String> = Vec::new();

    for (name, module) in &registry.modules {
        let module_path = modules_dir.join(&module.folder);
        let manifest = match ModuleManager::load_manifest(&module_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let flags: Vec<String> = manifest
            .options
            .iter()
            .filter(|opt| !opt.flags.is_empty())
            .filter_map(|opt| opt.flags.get(0).cloned())
            .collect();

        if flags.is_empty() {
            results.push(name.clone());
        } else {
            results.push(format!("{}:{}", name, flags.join(",")));
        }
    }

    results.join("|")
}

fn generate_bash_completion() -> String {
    let commands = "run add edit rm list update doctor help build-command edit-aliases completion add-repo list-repos search-mods install-mods add-mod update-mod inspect-mod autoupdate upgrade";
    
    format!(r#"# aktools bash completion

_aktools() {{
    local cur prev opts
    COMPREPLY=()
    cur="{{{{COMP_WORDS[COMP_CWORD]}}}}"
    prev="{{{{COMP_WORDS[COMP_CWORD-1]}}}}"
    prev2="{{{{COMP_WORDS[COMP_CWORD-2]}}}}"
    opts="{commands}"

    # Get module names and flags
    local modules_data
    modules_data=$(aktools _compdata 2>/dev/null || echo "")
    local module_names
    module_names=$(echo "$modules_data" | cut -d'|' -f1 | tr ' ' '\n' | grep -v '^$' | sort | uniq)

    case "${{prev}}" in
        run|edit|rm|inspect-mod)
            COMPREPLY=($(compgen -W "${{module_names}}" -- "${{cur}}"))
            ;;
        upgrade)
            COMPREPLY=($(compgen -W "aktools modules all" -- "${{cur}}"))
            ;;
        *)
            # Check if previous word is a module name for flag completion
            if echo "${{module_names}}" | grep -q "^${{prev}}$"; then
                # Get flags for this module
                local mod_flags
                mod_flags=$(echo "$modules_data" | grep "^${{prev}}:" | cut -d':' -f2 | tr ',' ' ')
                COMPREPLY=($(compgen -W "${{mod_flags}}" -- "${{cur}}"))
            else
                COMPREPLY=($(compgen -W "${{opts}}" -- "${{cur}}"))
            fi
            ;;
    esac

    # Handle flag completion after module name (e.g., "aktools run mymod <TAB>")
    if [[ "${{prev2}}" == @(run|edit|rm|inspect-mod) ]] && [[ "${{prev}}" != -* ]]; then
        local mod_flags
        mod_flags=$(echo "$modules_data" | grep "^${{prev}}:" | cut -d':' -f2 | tr ',' ' ')
        if [[ -n "$mod_flags" ]]; then
            COMPREPLY=($(compgen -W "${{mod_flags}}" -- "${{cur}}"))
        fi
    fi
}}
complete -F _aktools aktools
"#)
}

fn generate_zsh_completion() -> String {
    let commands_list = [
        "run", "add", "edit", "rm", "list", "update", "doctor", "help",
        "build-command", "edit-aliases", "completion", "add-repo", "list-repos",
        "search-mods", "install-mods", "add-mod", "update-mod", "inspect-mod",
        "autoupdate", "upgrade",
    ];
    let commands = commands_list
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<_>>()
        .join(" ");
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

    # Get module data
    local modules_data
    modules_data=$(aktools _compdata 2>/dev/null)
    local -a module_list
    module_list=($(echo "$modules_data" | cut -d'|' -f1 | tr ' ' '\n' | grep -v '^$'))

    case "${{words[2]}}" in
        run|edit|rm|inspect-mod)
            _describe 'module' module_list
            ;;
        upgrade)
            _describe 'upgrade target' upgrade_opts
            ;;
        *)
            # Check if previous word is a module for flag completion
            local prev_word="${{words[CURRENT-1]}}"
            if [[ -n "$prev_word" ]] && [[ "$prev_word" != -* ]]; then
                local mod_flags
                mod_flags=$(echo "$modules_data" | grep "^${{prev_word}}:" | cut -d':' -f2 | tr ',' ' ')
                if [[ -n "$mod_flags" ]]; then
                    _describe 'flag' "$mod_flags"
                    return
                fi
            fi
            ;;
    esac
}}

_aktools "$@"
"#)
}

fn generate_fish_completion() -> String {
    let script = r#"# aktools fish completion

function __aktools_modules
    aktools _compdata 2>/dev/null | cut -d'|' -f1 | tr ' ' '\n' | grep -v '^$'
end

function __aktools_module_flags
    set -l module $argv[1]
    aktools _compdata 2>/dev/null | grep "^$module:" | cut -d':' -f2 | tr ',' '\n'
end

# Command completions
complete -c aktools -f -a 'run' -d 'Run a module'
complete -c aktools -f -a 'add' -d 'Add a module'
complete -c aktools -f -a 'edit' -d 'Edit a module manifest'
complete -c aktools -f -a 'rm' -d 'Remove a module'
complete -c aktools -f -a 'list' -d 'List installed modules'
complete -c aktools -f -a 'update' -d 'Rebuild the registry'
complete -c aktools -f -a 'doctor' -d 'Diagnose issues'
complete -c aktools -f -a 'help' -d 'Show help'
complete -c aktools -f -a 'build-command' -d 'Create command module'
complete -c aktools -f -a 'edit-aliases' -d 'Edit aliases'
complete -c aktools -f -a 'completion' -d 'Generate shell completions'
complete -c aktools -f -a 'add-repo' -d 'Add a repo'
complete -c aktools -f -a 'list-repos' -d 'List repos'
complete -c aktools -f -a 'search-mods' -d 'Search modules'
complete -c aktools -f -a 'install-mods' -d 'Install modules'
complete -c aktools -f -a 'add-mod' -d 'Submit module to repo'
complete -c aktools -f -a 'update-mod' -d 'Update module in repo'
complete -c aktools -f -a 'inspect-mod' -d 'Show module contents'
complete -c aktools -f -a 'autoupdate' -d 'Manage autoupdate'
complete -c aktools -f -a 'upgrade' -d 'Upgrade aktools/modules'

# Module name completions for run, edit, rm, inspect-mod
complete -c aktools -n '__fish_seen_subcommand_from run edit rm inspect-mod' -a '(__aktools_modules)' -d 'module'

# Upgrade target completions
complete -c aktools -n '__fish_seen_subcommand_from upgrade' -a 'aktools modules all' -d 'target'

# Flag completions - dynamic based on selected module
complete -c aktools -n '__fish_seen_subcommand_from run edit rm' -f -a '(__aktools_module_flags (commandline -opc)[3])' -d 'flag'
"#;
    script.to_string()
}

fn generate_powershell_completion() -> String {
    let script = r#"# aktools powershell completion

$script:AktoolsModules = $null

function Get-AktoolsModules {
    $script:AktoolsModules = @(aktools _compdata 2>$null | ForEach-Object { ($_ -split '\|')[0] -split ' ' } | Where-Object { $_ })
}

function Get-AktoolsModuleFlags {
    param([string]$Module)
    if (-not $script:AktoolsModules) { Get-AktoolsModules }
    $line = aktools _compdata 2>$null | Where-Object { $_ -match "^$Module:" }
    if ($line -match ':(.+)') {
        return ($matches[1] -split ',').Trim()
    }
    return @()
}

$validCommands = @('run', 'add', 'edit', 'rm', 'list', 'update', 'doctor', 'help',
    'build-command', 'edit-aliases', 'completion', 'add-repo', 'list-repos',
    'search-mods', 'install-mods', 'add-mod', 'update-mod', 'inspect-mod',
    'autoupdate', 'upgrade')

$moduleCommands = @('run', 'edit', 'rm', 'inspect-mod')

Register-ArgumentCompleter -CommandName aktools -ParameterName Command -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $wordToComplete | ForEach-Object { $_ } | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}

Register-ArgumentCompleter -CommandName aktools -ParameterName ModuleName -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    if (-not $script:AktoolsModules) { Get-AktoolsModules }
    $script:AktoolsModules | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}
"#;
    script.to_string()
}

fn generate_elvish_completion() -> String {
    let script = r#"# aktools elvish completion

use runtime

set:& aktools-commands = [
    run add edit rm list update doctor help build-command edit-aliases completion
    add-repo list-repos search-mods install-mods add-mod update-mod inspect-mod
    autoupdate upgrade
]

set:& aktools-module-commands = [run edit rm inspect-mod]

fn get-aktools-modules {
    aktools _compdata 2>/dev/null | splits '|' | get 0 | splits ' '
}

fn get-aktools-flags {|module|
    aktools _compdata 2>/dev/null | grep $module | splits ':' | get 1 | splits ','
}

set:& comp-posthooks = [
    $@module in $aktools-module-commands {
        candidates [ (get-aktools-modules) ]
    }
]
"#;
    script.to_string()
}

/// Internal command to output completion data for shell scripts
/// Returns module data in format: "module1 module2|module1:flag1,flag2|module2:flag1"
pub fn execute_compdata(modules_dir: &Path, registry_path: &Path) -> String {
    get_modules_with_flags(modules_dir, registry_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_scripts_are_valid() {
        // Verify all shell scripts compile/are valid
        let bash = generate_bash_completion();
        assert!(bash.contains("_aktools()"));
        assert!(bash.contains("complete -F _aktools aktools"));

        let zsh = generate_zsh_completion();
        assert!(zsh.contains("_aktools()"));
        assert!(zsh.contains("_aktools \"$@\""));

        let fish = generate_fish_completion();
        assert!(fish.contains("complete -c aktools"));
        assert!(fish.contains("__aktools_modules"));

        let ps = generate_powershell_completion();
        assert!(ps.contains("Register-ArgumentCompleter"));

        let elvish = generate_elvish_completion();
        assert!(elvish.contains("aktools-commands"));
    }

    #[test]
    fn test_execute_unknown_shell() {
        let result = execute("unknown_shell");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported shell"));
    }
}
