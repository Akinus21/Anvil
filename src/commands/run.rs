use std::path::Path;
use std::process::{Command, Stdio};
use std::fs;
use crate::registry::Registry;

pub fn execute(modules_dir: &Path, registry_path: &Path, module_name: &str, args: Vec<String>) -> i32 {
    let registry = match Registry::load(registry_path) {
        Ok(r) => r,
        Err(e) => {
            println!("Error loading registry: {}", e);
            return 1;
        }
    };

    let module = match registry.modules.get(module_name) {
        Some(m) => m,
        None => {
            println!("Error: module '{}' not found", module_name);
            println!("Run 'anvil list' to see available modules.");
            return 1;
        }
    };

    let module_path = modules_dir.join(&module.folder);
    if !module_path.exists() {
        println!("Error: module folder not found at {:?}", module_path);
        return 1;
    }

    let manifest = match crate::modules::ModuleManager::load_manifest(&module_path) {
        Ok(m) => m,
        Err(e) => {
            println!("Error loading module manifest: {}", e);
            return 1;
        }
    };

    if !manifest.executable.is_empty() {
        let executable_path = module_path.join(&manifest.executable);
        let ext = executable_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut cmd = match ext.as_ref() {
            "py" => {
                let mut c = Command::new("python3");
                c.arg(&executable_path);
                c
            }
            "sh" | "bash" => {
                let mut c = Command::new("bash");
                c.arg(&executable_path);
                c
            }
            _ => Command::new(&executable_path),
        };

        cmd.args(&args);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        match cmd.spawn() {
            Ok(mut child) => child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1),
            Err(e) => {
                println!("Error executing module: {}", e);
                1
            }
        }
    } else {
        let matched_opt = if let Some(first_arg) = args.first() {
            manifest.options.iter().find(|o| o.flags.iter().any(|f| f.trim_start_matches('*') == first_arg))
        } else {
            None
        };

        let opt = match matched_opt {
            Some(o) => o,
            None => {
                println!("Error: no matching flag found for '{}'", args.first().unwrap_or(&"".to_string()));
                return 1;
            }
        };

        let flag_name = opt.flags.get(0).cloned().unwrap_or_else(|| "default".to_string());
        let script_path = module_path.join(format!("commands_{}.sh", flag_name));

        let has_shell_operators = opt.commands.iter().any(|cmd| {
            cmd.contains("&&") || cmd.contains("||") || cmd.contains(";") ||
            cmd.starts_with("sudo") || cmd.contains(" &") || cmd.ends_with(" &") ||
            cmd.trim_end().ends_with("&")
        });

        if has_shell_operators && !opt.commands.is_empty() {
            let script_content = format!("#!/bin/bash\nset -e\n\n{}\n", opt.commands.join("\n"));
            fs::write(&script_path, script_content).ok();
            let mut cmd = Command::new("bash");
            cmd.arg(&script_path);
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
            match cmd.spawn().and_then(|mut c| c.wait()) {
                Ok(exit_status) => {
                    if !exit_status.success() {
                        return 1;
                    }
                }
                Err(e) => {
                    println!("Error executing commands: {}", e);
                    return 1;
                }
            }
        } else {
            for cmd_str in &opt.commands {
                let mut parts = cmd_str.split_whitespace();
                let program = parts.next().unwrap_or("");
                let mut cmd = Command::new(program);
                cmd.args(parts);
                cmd.stdout(Stdio::inherit());
                cmd.stderr(Stdio::inherit());
                match cmd.spawn().and_then(|mut c| c.wait()) {
                    Ok(exit_status) => {
                        if !exit_status.success() {
                            return 1;
                        }
                    }
                    Err(e) => {
                        println!("Error executing command '{}': {}", cmd_str, e);
                        return 1;
                    }
                }
            }
        }
        0
    }
}
