use std::path::Path;
use crate::modules::ModuleManager;
use crate::registry::Registry;

/// Display general help for Anvil
pub fn show_general_help() {
    println!("Anvil - Modular CLI tool runner\n");
    println!("Commands:");
    println!("  anvil build-command   Create a new command module interactively");
    println!("  anvil add <file>     Add a script as a module");
    println!("  anvil edit [name]    Edit a module's manifest");
    println!("  anvil edit-aliases   Edit aliases interactively");
    println!("  anvil list           List installed modules");
    println!("  anvil rm <name>      Remove a module");
    println!("  anvil update         Rebuild the registry");
    println!("  anvil doctor         Diagnose and auto-fix issues");
    println!("  anvil completion     Generate shell completions");
    println!("  anvil add-repo       Add a GitHub repo to track");
    println!("  anvil list-repos     List configured repos");
    println!("  anvil search-mods    Search modules in repos");
    println!("  anvil install-mods   Install modules from repos");
    println!("  anvil add-mod        Submit module to community repo");
    println!("  anvil autoupdate     Manage automatic updates");
    println!("  anvil help <module>  Show help for a specific module");
    println!("  anvil <module> [args...]  Run a module");
}

/// Display help for a specific module
pub fn show_module_help(
    modules_dir: &Path,
    registry_path: &Path,
    module_name: &str,
) -> Result<(), String> {
    let registry = Registry::load(registry_path)
        .map_err(|e| format!("Error loading registry: {}", e))?;

    let module = registry.modules.get(module_name)
        .ok_or_else(|| format!("Module '{}' not found", module_name))?;

    let module_path = modules_dir.join(&module.folder);
    if !module_path.exists() {
        return Err(format!("Module folder not found: {:?}", module_path));
    }

    let manifest = ModuleManager::load_manifest(&module_path)
        .map_err(|e| format!("Error loading module manifest: {}", e))?;

    // Try to read description from README.md if it exists
    let description = read_module_description(&module_path, &manifest.name);

    println!("=== Module: {} ===\n", manifest.name);

    println!("Description: {}", description);

    println!("\nUsage: anvil run {} [options]", manifest.name);

    // Show aliases if available
    if !manifest.aliases.is_empty() && manifest.aliases.len() > 1 {
        println!("Aliases: {}", manifest.aliases.join(", "));
    }

    // Show executable info
    if !manifest.executable.is_empty() {
        println!("Executable: {}", manifest.executable);
    }

    // Show options
    if !manifest.options.is_empty() {
        println!("\nOptions:");
        for opt in &manifest.options {
            for flag in &opt.flags {
                let clean_flag = flag.trim_start_matches('*');
                let flag_display = if flag.starts_with('*') {
                    format!("--{}", clean_flag)
                } else {
                    format!("--{}", clean_flag)
                };

                if opt.commands.is_empty() {
                    println!("  {}          (no command defined)", flag_display);
                } else {
                    let cmd_preview = opt.commands.first()
                        .map(|c| if c.len() > 50 { format!("{}...", &c[..47]) } else { c.clone() })
                        .unwrap_or_else(|| "(empty)".to_string());
                    println!("  {:17} {}", flag_display, cmd_preview);
                }
            }
        }
    } else {
        println!("\nOptions: (none defined)");
    }

    // Show examples based on module type
    println!("\nExamples:");
    if !manifest.executable.is_empty() {
        println!("  anvil run {}          # Run with default script", manifest.name);
        println!("  anvil run {} --help    # Show this help", manifest.name);
    } else if !manifest.options.is_empty() {
        if let Some(first_opt) = manifest.options.first() {
            if let Some(first_flag) = first_opt.flags.first() {
                let clean_flag = first_flag.trim_start_matches('*');
                println!("  anvil run {} --{}    # Execute option", manifest.name, clean_flag);
            }
        }
        println!("  anvil help {}         # Show this help", manifest.name);
    } else {
        println!("  (no examples available)");
    }

    Ok(())
}

/// Read module description from README.md if it exists
fn read_module_description(module_path: &Path, _module_name: &str) -> String {
    let readme_path = module_path.join("README.md");
    if readme_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            // Extract first paragraph (lines until empty line)
            let first_para: String = content
                .lines()
                .take_while(|line| !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            // Remove markdown heading if present
            let description = first_para
                .trim_start_matches(|c: char| c == '#' || c == ' ')
                .trim();

            if !description.is_empty() {
                return description.to_string();
            }
        }
    }
    "No description available".to_string()
}

/// Execute the help command
pub fn execute(modules_dir: &Path, registry_path: &Path, args: &[String]) -> i32 {
    if let Some(module_name) = args.first() {
        match show_module_help(modules_dir, registry_path, module_name) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{}", e);
                1
            }
        }
    } else {
        show_general_help();
        0
    }
}
