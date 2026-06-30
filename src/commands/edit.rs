use std::io::{self, Write};
use std::path::Path;

use crate::modules::{ModuleManager, ModuleManifest, OptionSwitch};

pub fn execute(modules_dir: &Path, _registry_path: &Path, module_name: Option<String>) -> i32 {
    if std::io::IsTerminal::is_terminal(&std::io::stdin())
        && std::io::IsTerminal::is_terminal(&std::io::stdout())
    {
        crate::commands::edit_tui::run(modules_dir, module_name)
    } else {
        // Phase 1: Load the module list for selection
        let modules = match ModuleManager::scan_modules(modules_dir) {
            Ok(m) => m,
            Err(e) => {
                println!("Error scanning modules: {}", e);
                return 1;
            }
        };

        // Phase 2: Select which module to edit
        let selected_name = if let Some(name) = module_name {
            if !modules.contains_key(&name) {
                println!("Error: module '{}' not found", name);
                return 1;
            }
            name
        } else {
            match select_module_from_list(&modules) {
                Some(name) => name,
                None => return 0,
            }
        };

        // Phase 3: Load the manifest ONCE into memory
        let module_path = modules_dir.join(&selected_name);
        let mut manifest = match ModuleManager::load_manifest(&module_path) {
            Ok(m) => m,
            Err(e) => {
                println!("Error loading module manifest: {}", e);
                return 1;
            }
        };

        // Track if we've made changes
        let mut dirty = false;

        // Phase 4: Main edit loop - all modifications are in-memory
        loop {
            println!("\n=== Editing: {} ===", manifest.name);
            println!("1. Name: {}", manifest.name);
            println!("2. Aliases: {:?}", manifest.aliases);
            println!("3. Executable: {}", if manifest.executable.is_empty() { "(none - command-only)" } else { &manifest.executable });
            println!("4. Options: {} option(s)", manifest.options.len());
            for (i, opt) in manifest.options.iter().enumerate() {
                println!("   Option {}: flags={:?}, {} command(s)", i + 1, opt.flags, opt.commands.len());
            }
            println!("\ns. Save and quit");
            println!("q. Quit{}", if dirty { " (unsaved changes)" } else { "" });

            print!("\nSelect field to edit (1-4), 's' to save, or 'q' to quit: ");
            if let Err(_) = io::stdout().flush() {
                continue;
            }

            let mut input = String::new();
            if let Err(_) = io::stdin().read_line(&mut input) {
                println!("Error reading input");
                continue;
            }

            let input = input.trim();
            match input {
                "q" => {
                    if dirty {
                        print!("You have unsaved changes. Save before quitting? [y/n]: ");
                        if let Err(_) = io::stdout().flush() {
                            continue;
                        }
                        let mut confirm = String::new();
                        if let Err(_) = io::stdin().read_line(&mut confirm) {
                            continue;
                        }
                        if confirm.trim().eq_ignore_ascii_case("y") {
                            if let Err(e) = save_manifest(&module_path, &manifest) {
                                println!("Error saving manifest: {}", e);
                                continue;
                            }
                            println!("Manifest saved successfully.");
                        }
                    }
                    return 0;
                }
                "s" => {
                    if let Err(e) = save_manifest(&module_path, &manifest) {
                        println!("Error saving manifest: {}", e);
                        continue;
                    }
                    println!("Manifest saved successfully.");

                    // Verify by reloading
                    match ModuleManager::load_manifest(&module_path) {
                        Ok(verified) => {
                            if verified.name != manifest.name
                                || verified.aliases != manifest.aliases
                                || verified.executable != manifest.executable
                                || verified.options.len() != manifest.options.len()
                            {
                                println!("Warning: Verification failed. The saved file differs from expected.");
                            } else {
                                println!("Verification: OK");
                            }
                        }
                        Err(e) => println!("Warning: Could not verify save: {}", e),
                    }
                    return 0;
                }
                "1" => {
                    if let Err(e) = edit_name(&mut manifest, &module_path, modules_dir) {
                        println!("Error: {}", e);
                    } else {
                        dirty = true;
                    }
                }
                "2" => {
                    if let Err(e) = edit_aliases(&mut manifest) {
                        println!("Error: {}", e);
                    } else {
                        dirty = true;
                    }
                }
                "3" => {
                    if let Err(e) = edit_executable(&mut manifest) {
                        println!("Error: {}", e);
                    } else {
                        dirty = true;
                    }
                }
                "4" => {
                    if let Err(e) = edit_options(&mut manifest, &module_path) {
                        println!("Error: {}", e);
                    } else {
                        dirty = true;
                    }
                }
                _ => {
                    println!("Invalid selection");
                }
            }
        }
    }
}

fn select_module_from_list(modules: &std::collections::HashMap<String, ModuleManifest>) -> Option<String> {
    loop {
        println!("\nInstalled modules:");
        let mut names: Vec<_> = modules.keys().collect();
        names.sort();
        for (i, name) in names.iter().enumerate() {
            println!("  {} - {}", i + 1, name);
        }
        println!("  q - quit");

        print!("\nSelect module to edit: ");
        if let Err(_) = io::stdout().flush() {
            continue;
        }
        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("Error reading input");
            continue;
        }

        let input = input.trim();
        if input == "q" {
            return None;
        }

        if let Ok(idx) = input.parse::<usize>() {
            if idx > 0 && idx <= names.len() {
                return Some(names[idx - 1].clone());
            }
        }
        println!("Invalid selection");
    }
}

fn save_manifest(module_path: &Path, manifest: &ModuleManifest) -> io::Result<()> {
    ModuleManager::write_manifest(module_path, manifest)
}

fn edit_name(manifest: &mut ModuleManifest, _module_path: &Path, modules_dir: &Path) -> io::Result<()> {
    print!("New name [current: {}]: ", manifest.name);
    if let Err(_) = io::stdout().flush() {
        return Ok(());
    }

    let mut input = String::new();
    if let Err(_) = io::stdin().read_line(&mut input) {
        return Err(io::Error::new(io::ErrorKind::Other, "Error reading input"));
    }

    let new_name = input.trim().to_string();

    // Validate: name cannot be empty
    if new_name.is_empty() {
        println!("Name cannot be empty. Keeping: {}", manifest.name);
        return Ok(());
    }

    if new_name == manifest.name {
        return Ok(());
    }

    // Rename the module folder if it exists
    let old_module_dir = modules_dir.join(&manifest.name);
    let new_module_dir = modules_dir.join(&new_name);

    if old_module_dir.exists() {
        if new_module_dir.exists() {
            println!("Error: a module named '{}' already exists.", new_name);
            return Ok(());
        }
        std::fs::rename(&old_module_dir, &new_module_dir)?;
    }

    manifest.name = new_name;
    println!("Name updated.");
    Ok(())
}

fn edit_aliases(manifest: &mut ModuleManifest) -> io::Result<()> {
    let aliases_str = manifest.aliases.join(", ");
    print!("New aliases (comma-separated) [current: {}]: ", aliases_str);
    if let Err(_) = io::stdout().flush() {
        return Ok(());
    }

    let mut input = String::new();
    if let Err(_) = io::stdin().read_line(&mut input) {
        return Err(io::Error::new(io::ErrorKind::Other, "Error reading input"));
    }

    let input = input.trim();
    if input.is_empty() {
        println!("Aliases unchanged.");
        return Ok(());
    }

    // Parse comma-separated aliases, trim whitespace
    let new_aliases: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if new_aliases.is_empty() {
        println!("No valid aliases provided. Keeping old value.");
        return Ok(());
    }

    // Check for duplicates
    let mut unique = new_aliases.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != new_aliases.len() {
        println!("Warning: duplicate aliases removed.");
    }

    manifest.aliases = unique;
    println!("Aliases updated.");
    Ok(())
}

fn edit_executable(manifest: &mut ModuleManifest) -> io::Result<()> {
    let current = if manifest.executable.is_empty() {
        "(none - command-only module)".to_string()
    } else {
        manifest.executable.clone()
    };
    print!("New executable path [current: {}]: ", current);
    if let Err(_) = io::stdout().flush() {
        return Ok(());
    }

    let mut input = String::new();
    if let Err(_) = io::stdin().read_line(&mut input) {
        return Err(io::Error::new(io::ErrorKind::Other, "Error reading input"));
    }

    // Empty is allowed for command-only modules
    manifest.executable = input.trim().to_string();
    println!("Executable updated.");
    Ok(())
}

fn edit_options(manifest: &mut ModuleManifest, _module_path: &Path) -> io::Result<()> {
    loop {
        println!("\n=== Options ===");
        if manifest.options.is_empty() {
            println!("No options defined.");
        } else {
            for (i, opt) in manifest.options.iter().enumerate() {
                println!("{}. Flags: {:?}, Commands: {}", i + 1, opt.flags, opt.commands.len());
                for (j, cmd) in opt.commands.iter().enumerate() {
                    println!("   {}.{}: {}", i + 1, j + 1, cmd);
                }
            }
        }
        println!("\na. Add new option");
        if !manifest.options.is_empty() {
            println!("d. Delete option");
        }
        println!("q. Back to main menu");

        let prompt = if manifest.options.is_empty() {
            "\nSelect action [a/q]: "
        } else {
            "\nSelect option (1-{}), 'a' to add, 'd' to delete, 'q' to go back: "
        };
        let max_opt = manifest.options.len();

        print!("{}", prompt.replace("{}", &max_opt.to_string()));
        if let Err(_) = io::stdout().flush() {
            continue;
        }

        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("Error reading input");
            continue;
        }

        let input = input.trim();
        match input {
            "q" => return Ok(()),
            "a" => {
                if let Err(e) = add_option(manifest) {
                    println!("Error adding option: {}", e);
                }
            }
            "d" => {
                if manifest.options.len() > 1 {
                    if let Err(e) = delete_option(manifest) {
                        println!("Error deleting option: {}", e);
                    }
                } else {
                    println!("Cannot delete the last option.");
                }
            }
            _ => {
                if let Ok(idx) = input.parse::<usize>() {
                    if idx > 0 && idx <= manifest.options.len() {
                        if let Err(e) = edit_single_option(manifest, idx - 1) {
                            println!("Error editing option: {}", e);
                        }
                    } else {
                        println!("Invalid selection");
                    }
                } else {
                    // Try to select by flag name
                    if let Some(opt_idx) = find_option_by_flag(manifest, input) {
                        if let Err(e) = edit_single_option(manifest, opt_idx) {
                            println!("Error editing option: {}", e);
                        }
                    } else {
                        println!("Invalid selection. Enter a number (1-{}) or a flag name.", manifest.options.len());
                    }
                }
            }
        }
    }
}

fn find_option_by_flag(manifest: &ModuleManifest, flag_input: &str) -> Option<usize> {
    for (i, opt) in manifest.options.iter().enumerate() {
        if opt.flags.iter().any(|f| f == flag_input) {
            return Some(i);
        }
    }
    None
}

fn add_option(manifest: &mut ModuleManifest) -> io::Result<()> {
    print!("Enter flag name: ");
    if let Err(_) = io::stdout().flush() {
        return Ok(());
    }

    let mut flag_input = String::new();
    if let Err(_) = io::stdin().read_line(&mut flag_input) {
        return Err(io::Error::new(io::ErrorKind::Other, "Error reading input"));
    }

    let flag = flag_input.trim().to_string();

    // Validate: no empty flags
    if flag.is_empty() {
        println!("Flag cannot be empty.");
        return Ok(());
    }

    // Check for duplicate flags within existing options
    for opt in &manifest.options {
        if opt.flags.contains(&flag) {
            println!("Error: flag '{}' already exists in another option.", flag);
            return Ok(());
        }
    }

    print!("Enter command: ");
    if let Err(_) = io::stdout().flush() {
        return Ok(());
    }

    let mut cmd_input = String::new();
    if let Err(_) = io::stdin().read_line(&mut cmd_input) {
        return Err(io::Error::new(io::ErrorKind::Other, "Error reading input"));
    }

    let command = cmd_input.trim().to_string();

    // Validate: command cannot be empty
    if command.is_empty() {
        println!("Command cannot be empty.");
        return Ok(());
    }

    // Warn about shell operators
    let test_commands = vec![command.clone()];
    if ModuleManager::has_shell_operators(&test_commands) {
        println!("Warning: command contains shell operators (&&, ||, ;, sudo, &). This may be dangerous.");
    }

    let new_option = OptionSwitch {
        flags: vec![flag],
        _is_default: false,
        commands: vec![command],
    };

    manifest.options.push(new_option);
    println!("Option added.");
    Ok(())
}

fn delete_option(manifest: &mut ModuleManifest) -> io::Result<()> {
    print!("Select option to delete (1-{}): ", manifest.options.len());
    if let Err(_) = io::stdout().flush() {
        return Ok(());
    }

    let mut input = String::new();
    if let Err(_) = io::stdin().read_line(&mut input) {
        return Err(io::Error::new(io::ErrorKind::Other, "Error reading input"));
    }

    if let Ok(idx) = input.trim().parse::<usize>() {
        if idx > 0 && idx <= manifest.options.len() {
            manifest.options.remove(idx - 1);
            println!("Option deleted.");
        } else {
            println!("Invalid selection.");
        }
    } else {
        println!("Invalid input.");
    }
    Ok(())
}

fn edit_single_option(manifest: &mut ModuleManifest, opt_idx: usize) -> io::Result<()> {
    let option = &manifest.options[opt_idx];
    println!("\n=== Editing Option {} ===", opt_idx + 1);
    println!("Flags: {:?}", option.flags);
    println!("Commands: {}", option.commands.len());

    loop {
        println!("\n1. Edit flags");
        println!("2. View/edit commands");
        println!("q. Back to options");

        print!("\nSelect: ");
        if let Err(_) = io::stdout().flush() {
            continue;
        }

        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("Error reading input");
            continue;
        }

        let input = input.trim();
        match input {
            "q" => return Ok(()),
            "1" => {
                if let Err(e) = edit_option_flags(manifest, opt_idx) {
                    println!("Error editing flags: {}", e);
                }
            }
            "2" => {
                if let Err(e) = edit_option_commands(manifest, opt_idx) {
                    println!("Error editing commands: {}", e);
                }
            }
            _ => {
                println!("Invalid selection");
            }
        }
    }
}

fn edit_option_flags(manifest: &mut ModuleManifest, opt_idx: usize) -> io::Result<()> {
    loop {
        // Get current flags as an owned copy for display and duplicate checking
        let current_flags: Vec<String> = manifest.options.get(opt_idx)
            .map(|opt| opt.flags.clone())
            .unwrap_or_default();

        println!("\nCurrent flags: {:?}", current_flags);
        println!("a. Add flag");
        if current_flags.len() > 1 {
            println!("d. Delete flag");
        }
        println!("q. Back");

        print!("\nSelect: ");
        if let Err(_) = io::stdout().flush() {
            continue;
        }

        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("Error reading input");
            continue;
        }

        let input = input.trim();
        match input {
            "q" => return Ok(()),
            "a" => {
                print!("Enter new flag: ");
                if let Err(_) = io::stdout().flush() {
                    continue;
                }
                let mut flag_input = String::new();
                if let Err(_) = io::stdin().read_line(&mut flag_input) {
                    continue;
                }
                let new_flag = flag_input.trim().to_string();

                if new_flag.is_empty() {
                    println!("Flag cannot be empty.");
                    continue;
                }

                // Check for duplicate in other options
                let duplicate_in_other = manifest.options.iter()
                    .enumerate()
                    .any(|(i, opt)| i != opt_idx && opt.flags.contains(&new_flag));

                if duplicate_in_other {
                    println!("Flag '{}' already exists in another option.", new_flag);
                    continue;
                }

                // Add to this option
                manifest.options[opt_idx].flags.push(new_flag);
                println!("Flag added.");
            }
            "d" => {
                if current_flags.len() <= 1 {
                    println!("Cannot delete the last flag.");
                    continue;
                }
                print!("Select flag to delete (1-{}): ", current_flags.len());
                if let Err(_) = io::stdout().flush() {
                    continue;
                }
                let mut del_input = String::new();
                if let Err(_) = io::stdin().read_line(&mut del_input) {
                    continue;
                }
                if let Ok(idx) = del_input.trim().parse::<usize>() {
                    if idx > 0 && idx <= current_flags.len() {
                        manifest.options[opt_idx].flags.remove(idx - 1);
                        println!("Flag deleted.");
                    } else {
                        println!("Invalid selection.");
                    }
                }
            }
            _ => {
                println!("Invalid selection");
            }
        }
    }
}

fn edit_option_commands(manifest: &mut ModuleManifest, opt_idx: usize) -> io::Result<()> {
    let option = &mut manifest.options[opt_idx];

    loop {
        println!("\nCommands for option {}:", opt_idx + 1);
        if option.commands.is_empty() {
            println!("  (no commands)");
        } else {
            for (i, cmd) in option.commands.iter().enumerate() {
                println!("  {}. {}", i + 1, cmd);
            }
        }
        println!("\na. Add command");
        if !option.commands.is_empty() {
            println!("d. Delete command");
        }
        println!("q. Back");

        print!("\nSelect: ");
        if let Err(_) = io::stdout().flush() {
            continue;
        }

        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("Error reading input");
            continue;
        }

        let input = input.trim();
        match input {
            "q" => return Ok(()),
            "a" => {
                print!("Enter new command: ");
                if let Err(_) = io::stdout().flush() {
                    continue;
                }
                let mut cmd_input = String::new();
                if let Err(_) = io::stdin().read_line(&mut cmd_input) {
                    continue;
                }
                let new_cmd = cmd_input.trim().to_string();

                if new_cmd.is_empty() {
                    println!("Command cannot be empty.");
                    continue;
                }

                // Warn about shell operators
                let test_commands = vec![new_cmd.clone()];
                if ModuleManager::has_shell_operators(&test_commands) {
                    println!("Warning: command contains shell operators (&&, ||, ;, sudo, &). This may be dangerous.");
                }

                option.commands.push(new_cmd);
                println!("Command added.");
            }
            "d" => {
                if option.commands.is_empty() {
                    println!("No commands to delete.");
                    continue;
                }
                print!("Select command to delete (1-{}): ", option.commands.len());
                if let Err(_) = io::stdout().flush() {
                    continue;
                }
                let mut del_input = String::new();
                if let Err(_) = io::stdin().read_line(&mut del_input) {
                    continue;
                }
                if let Ok(idx) = del_input.trim().parse::<usize>() {
                    if idx > 0 && idx <= option.commands.len() {
                        option.commands.remove(idx - 1);
                        println!("Command deleted.");
                    } else {
                        println!("Invalid selection.");
                    }
                }
            }
            _ => {
                // Try to edit a specific command
                if let Ok(cmd_idx) = input.parse::<usize>() {
                    if cmd_idx > 0 && cmd_idx <= option.commands.len() {
                        let old_cmd = &option.commands[cmd_idx - 1];
                        print!("Enter new command [old: {}]: ", old_cmd);
                        if let Err(_) = io::stdout().flush() {
                            continue;
                        }
                        let mut new_cmd_input = String::new();
                        if let Err(_) = io::stdin().read_line(&mut new_cmd_input) {
                            continue;
                        }
                        let new_cmd = new_cmd_input.trim().to_string();

                        if new_cmd.is_empty() {
                            println!("Command unchanged.");
                            continue;
                        }

                        // Warn about shell operators
                        let test_commands = vec![new_cmd.clone()];
                        if ModuleManager::has_shell_operators(&test_commands) {
                            println!("Warning: command contains shell operators (&&, ||, ;, sudo, &). This may be dangerous.");
                        }

                        option.commands[cmd_idx - 1] = new_cmd;
                        println!("Command updated.");
                    } else {
                        println!("Invalid selection.");
                    }
                } else {
                    println!("Invalid selection. Enter a number, 'a', 'd', or 'q'.");
                }
            }
        }
    }
}