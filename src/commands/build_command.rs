use std::io::{self, Write};
use std::path::Path;

pub fn execute(modules_dir: &Path, registry_path: &Path) -> i32 {
    println!("Anvil - Build Command Module\n");
    println!("Press 'q' at any prompt to cancel.\n");

    print!("Module name: ");
    io::stdout().flush().unwrap();
    let mut name = String::new();
    if io::stdin().read_line(&mut name).is_err() || name.trim() == "q" {
        println!("Cancelled.");
        return 1;
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("Error: module name cannot be empty.");
        return 1;
    }

    print!("Aliases (comma-separated): ");
    io::stdout().flush().unwrap();
    let mut aliases_input = String::new();
    if io::stdin().read_line(&mut aliases_input).is_err() || aliases_input.trim() == "q" {
        println!("Cancelled.");
        return 1;
    }
    let aliases: Vec<String> = aliases_input
        .trim()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut options: Vec<(String, String)> = Vec::new();

    loop {
        print!("\nFlag (or 'q' to finish, <ENTER> for no-flag command): ");
        io::stdout().flush().unwrap();
        let mut flag = String::new();
        if io::stdin().read_line(&mut flag).is_err() || flag.trim() == "q" {
            break;
        }
        let flag = flag.trim().to_string();
        if flag.is_empty() {
            print!("No-flag command will be executed when module is called without flags. Continue? (y/n): ");
            io::stdout().flush().unwrap();
            let mut confirm = String::new();
            if io::stdin().read_line(&mut confirm).is_err() {
                println!("Error reading input.");
                continue;
            }
            if confirm.trim().to_lowercase() != "y" {
                println!("Cancelled.");
                continue;
            }
            println!("Note: Commands will be saved to <module_name>.sh instead of flag-specific files.");
        }

        print!("Command: ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        if io::stdin().read_line(&mut command).is_err() || command.trim() == "q" {
            println!("Cancelled.");
            return 1;
        }
        let command = command.trim().to_string();
        if command.is_empty() {
            println!("Command cannot be empty.");
            continue;
        }

        options.push((flag.clone(), command.clone()));
        if flag.is_empty() {
            println!("Added: (no flag) -> {}", command);
        } else {
            println!("Added: {} -> {}", flag, command);
        }
    }

    if options.is_empty() {
        println!("Error: at least one flag/command pair is required.");
        return 1;
    }

    let has_no_flag = options.iter().any(|(f, _)| f.is_empty());
    let script_name = if has_no_flag {
        format!("{}.sh", name)
    } else {
        String::from("commands.sh")
    };

    let module_dir = modules_dir.join(&name);
    if module_dir.exists() {
        println!("Error: module '{}' already exists at {:?}", name, module_dir);
        return 1;
    }

    if let Err(e) = std::fs::create_dir_all(&module_dir) {
        println!("Error creating module directory: {}", e);
        return 1;
    }

    let mut manifest = format!(r#"<?xml version="1.0"?>
<module>
    <name>{}</name>
"#,
        name);

    if !aliases.is_empty() {
        manifest.push_str("    <alias>{}</alias>\n");
    }

    if has_no_flag {
        manifest.push_str(&format!("    <executable>{}</executable>\n", script_name));
    } else {
        manifest.push_str("    <executable></executable>\n");
    }

    for (flag, command) in &options {
        if flag.is_empty() {
            manifest.push_str(&format!(r#"    <option>
        <flag></flag>
        <command>{}</command>
    </option>
"#, command));
        } else {
            manifest.push_str(&format!(r#"    <option>
        <flag>{}</flag>
        <command>{}</command>
    </option>
"#, flag, command));
        }
    }

    manifest.push_str("</module>\n");

    if let Err(e) = std::fs::write(module_dir.join("manifest.xml"), &manifest) {
        println!("Error writing manifest.xml: {}", e);
        return 1;
    }

    let readme_content = format!("# {}\n\nDescribe what this module does here.\n", name);
    if let Err(e) = std::fs::write(module_dir.join("README.md"), &readme_content) {
        println!("Error writing README.md: {}", e);
    }

    if has_no_flag {
        let no_flag_commands: Vec<String> = options.iter()
            .filter(|(f, _)| f.is_empty())
            .map(|(_, c)| c.clone())
            .collect();
        let script_content = format!("#!/bin/bash\nset -e\n\n{}\n", no_flag_commands.join("\n"));
        if let Err(e) = std::fs::write(module_dir.join(&script_name), &script_content) {
            println!("Error writing script: {}", e);
            return 1;
        }
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(module_dir.join(&script_name), std::fs::Permissions::from_mode(0o755)) {
            println!("Error setting script permissions: {}", e);
            return 1;
        }
    }

    let has_no_flag = options.iter().any(|(f, _)| f.is_empty());
    let script_name = if has_no_flag {
        format!("{}.sh", name)
    } else {
        String::from("commands.sh")
    };

    let mut manifest = format!(r#"<?xml version="1.0"?>
<module>
    <name>{}</name>
"#,
        name);

    if !aliases.is_empty() {
        manifest.push_str("    <alias>{}</alias>\n");
    }

    if has_no_flag {
        manifest.push_str(&format!("    <executable>{}</executable>\n", script_name));
    } else {
        manifest.push_str("    <executable></executable>\n");
    }

    for (flag, command) in &options {
        if flag.is_empty() {
            manifest.push_str(&format!(r#"    <option>
        <flag></flag>
        <command>{}</command>
    </option>
"#, command));
        } else {
            manifest.push_str(&format!(r#"    <option>
        <flag>{}</flag>
        <command>{}</command>
    </option>
"#, flag, command));
        }
    }

    manifest.push_str("</module>\n");

    if let Err(e) = std::fs::write(module_dir.join("manifest.xml"), &manifest) {
        println!("Error writing manifest.xml: {}", e);
        return 1;
    }

    if let Err(e) = crate::registry::Registry::load(registry_path)
        .and_then(|mut r| {
            let module = crate::registry::Module {
                name: name.clone(),
                folder: name.clone(),
                aliases,
                commands: options.iter()
                    .map(|(f, c)| (f.clone(), vec![c.clone()]))
                    .collect(),
            };
            r.add_module(module);
            r.save(registry_path)
        }) {
        println!("Error updating registry: {}", e);
    }

    println!("\nCreated module '{}' at {:?}", name, module_dir);
    0
}