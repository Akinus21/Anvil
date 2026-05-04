use std::path::Path;

pub fn execute(config_dir: &Path, modules_dir: &Path, no_fix: bool) -> i32 {
    println!("AKTools Doctor - Diagnosing issues...\n");

    let mut issues_found = 0;
    let mut fixed = 0;

    println!("=== Config Directory ===");
    if !config_dir.exists() {
        if !no_fix {
            match std::fs::create_dir_all(config_dir) {
                Ok(_) => {
                    println!("  [FIXED] Created config directory: {:?}", config_dir);
                    fixed += 1;
                }
                Err(e) => {
                    println!("  [ERROR] Failed to create config directory: {}", e);
                    issues_found += 1;
                }
            }
        } else {
            println!("  [WARN] Config directory does not exist: {:?}", config_dir);
            println!("         Run without --no-fix to create it automatically");
            issues_found += 1;
        }
    } else {
        println!("  [OK] Config directory exists");
    }

    if !modules_dir.exists() {
        if !no_fix {
            match std::fs::create_dir_all(modules_dir) {
                Ok(_) => {
                    println!("  [FIXED] Created modules directory: {:?}", modules_dir);
                    fixed += 1;
                }
                Err(e) => {
                    println!("  [ERROR] Failed to create modules directory: {}", e);
                    issues_found += 1;
                }
            }
        } else {
            println!("  [WARN] Modules directory does not exist: {:?}", modules_dir);
            println!("         Run without --no-fix to create it automatically");
            issues_found += 1;
        }
    } else {
        println!("  [OK] Modules directory exists");
    }

    println!("\n=== Shell Configuration ===");
    let home = dirs::home_dir().unwrap_or_default();
    let shell_files = vec![
        home.join(".bashrc"),
        home.join(".zshrc"),
    ];

    let mut aktools_in_shell = false;
    let mut aliases_sourced = false;
    for shell_file in &shell_files {
        if shell_file.exists() {
            let content = std::fs::read_to_string(shell_file).unwrap_or_default();
            if content.contains("aktools") {
                println!("  [OK] aktools found in {:?}", shell_file);
                aktools_in_shell = true;
            }
            if content.contains("aliases.sh") {
                aliases_sourced = true;
            }
        }
    }

    if !aktools_in_shell && !no_fix {
        let export_line = format!(r#"# AKTools
export AKTOOLS_HOME="{}"
export PATH="$AKTOOLS_HOME/bin:$PATH"
source "$AKTOOLS_HOME/aliases.sh"
"#, config_dir.display());
        for shell_file in &shell_files {
            if shell_file.exists() {
                match std::fs::read_to_string(shell_file) {
                    Ok(content) => {
                        if !content.contains("aktools") {
                            match std::fs::write(shell_file, content + &export_line) {
                                Ok(_) => {
                                    println!("  [FIXED] Added AKTools to {:?}", shell_file);
                                    fixed += 1;
                                    aktools_in_shell = true;
                                    aliases_sourced = true;
                                }
                                Err(e) => {
                                    println!("  [ERROR] Failed to update {:?}: {}", shell_file, e);
                                    issues_found += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("  [ERROR] Failed to read {:?}: {}", shell_file, e);
                        issues_found += 1;
                    }
                }
            }
        }
    }

    if !aliases_sourced && !no_fix && aktools_in_shell {
        let source_line = "source \"$AKTOOLS_HOME/aliases.sh\"\n";
        for shell_file in &shell_files {
            if shell_file.exists() {
                match std::fs::read_to_string(shell_file) {
                    Ok(content) => {
                        if content.contains("aktools") && !content.contains("aliases.sh") {
                            let new_content = content.trim_end().to_string() + "\n" + source_line;
                            match std::fs::write(shell_file, new_content) {
                                Ok(_) => {
                                    println!("  [FIXED] Added aliases sourcing to {:?}", shell_file);
                                    fixed += 1;
                                }
                                Err(e) => {
                                    println!("  [ERROR] Failed to update {:?}: {}", shell_file, e);
                                    issues_found += 1;
                                }
                            }
                            break;
                        }
                    }
                    Err(e) => {
                        println!("  [ERROR] Failed to read {:?}: {}", shell_file, e);
                        issues_found += 1;
                    }
                }
            }
        }
    } else if !aktools_in_shell {
        println!("  [WARN] AKTools not found in shell configuration");
        println!("         Run without --no-fix to add it automatically");
        issues_found += 1;
    }

    println!("\n=== Autoupdate Scheduler ===");
    let scheduler = detect_scheduler();
    println!("  Detected scheduler: {}", scheduler);

    match scheduler {
        "systemd" => check_systemd_autoupdate(config_dir, no_fix, &mut fixed, &mut issues_found),
        "launchd" => check_launchd_autoupdate(no_fix, &mut fixed, &mut issues_found),
        "cron" => check_cron_autoupdate(no_fix, &mut fixed, &mut issues_found),
        _ => println!("  [OK] No autoupdate scheduler detected"),
    }

    println!("\n=== Aliases File ===");
    let aliases_file = config_dir.join("aliases.sh");
    if aliases_file.exists() {
        println!("  [OK] Aliases file exists: {:?}", aliases_file);
    } else {
        if !no_fix {
            let default_aliases = r#"# AKTools Aliases
# This file is auto-generated

alias ak='aktools'
alias akt='aktools'
alias aktools-update='aktools update'
alias aktools-doctor='aktools doctor'
alias aktools-add='aktools add'
alias aktools-rm='aktools rm'
alias aktools-edit='aktools edit'
"#;
            match std::fs::write(&aliases_file, default_aliases) {
                Ok(_) => {
                    println!("  [FIXED] Created aliases file: {:?}", aliases_file);
                    fixed += 1;
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o755);
                    let _ = std::fs::set_permissions(&aliases_file, perms);
                }
                Err(e) => {
                    println!("  [ERROR] Failed to create aliases file: {}", e);
                    issues_found += 1;
                }
            }
        } else {
            println!("  [WARN] Aliases file not found: {:?}", aliases_file);
            println!("         Run without --no-fix to create it automatically");
            issues_found += 1;
        }
    }

    println!("\n=== Registry File ===");
    let registry_file = config_dir.join("registry.json");
    if !registry_file.exists() {
        if !no_fix {
            match std::fs::write(&registry_file, "{\"modules\": []}") {
                Ok(_) => {
                    println!("  [FIXED] Created registry file: {:?}", registry_file);
                    fixed += 1;
                }
                Err(e) => {
                    println!("  [ERROR] Failed to create registry: {}", e);
                    issues_found += 1;
                }
            }
        } else {
            println!("  [WARN] Registry file not found: {:?}", registry_file);
            println!("         Run without --no-fix to create it automatically");
            issues_found += 1;
        }
    } else {
        println!("  [OK] Registry file exists");
    }

    println!("\n=== Repos Config ===");
    let repos_file = config_dir.join("repos.json");
    if !repos_file.exists() {
        if !no_fix {
            match std::fs::write(&repos_file, "{\"repos\": []}") {
                Ok(_) => {
                    println!("  [FIXED] Created repos config: {:?}", repos_file);
                    fixed += 1;
                }
                Err(e) => {
                    println!("  [ERROR] Failed to create repos config: {}", e);
                    issues_found += 1;
                }
            }
        } else {
            println!("  [WARN] Repos config not found: {:?}", repos_file);
            println!("         Run without --no-fix to create it automatically");
            issues_found += 1;
        }
    } else {
        println!("  [OK] Repos config exists");
    }

    println!("\n=== Binary Installation ===");
    let bin_dir = config_dir.join("bin");
    let aktools_bin = bin_dir.join("aktools");
    if aktools_bin.exists() {
        if let Ok(metadata) = std::fs::metadata(&aktools_bin) {
            println!("  [OK] aktools binary found: {} bytes", metadata.len());
        }
    } else {
        if !no_fix {
            let _ = std::process::Command::new("brew")
                .args(["update"])
                .output();

            let aktools_installed = std::process::Command::new("brew")
                .args(["list", "aktools"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if aktools_installed {
                let upgrade_result = std::process::Command::new("brew")
                    .args(["upgrade", "aktools"])
                    .output();

                match upgrade_result {
                    Ok(output) => {
                        if output.status.success() {
                            println!("  [FIXED] aktools upgraded via Homebrew");
                            fixed += 1;
                        } else {
                            println!("  [WARN] upgrade failed, use 'brew upgrade aktools' manually");
                            issues_found += 1;
                        }
                    }
                    Err(e) => {
                        println!("  [WARN] failed to run brew upgrade: {}", e);
                        issues_found += 1;
                    }
                }
            } else {
                println!("  [WARN] aktools not installed via Homebrew");
                println!("         Run 'brew install aktools' to install");
                issues_found += 1;
            }
        } else {
            println!("  [WARN] aktools binary not found in {:?}", aktools_bin);
            println!("         Run without --no-fix to auto-install via Homebrew");
            issues_found += 1;
        }
    }

    println!("\n=== GitHub Update Check ===");
    if let Ok(response) = ureq::get("https://api.github.com/repos/Akinus21/aktools/releases/latest")
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        if let Ok(body) = response.into_string() {
            if let Ok(releases) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(tag_name) = releases.get("tag_name").and_then(|t| t.as_str()) {
                    let current_version = env!("CARGO_PKG_VERSION");
                    if tag_name != format!("v{}", current_version) {
                        println!("  [INFO] Update available: {} -> {}", current_version, tag_name);
                    } else {
                        println!("  [OK] aktools is up to date (v{})", current_version);
                    }
                }
            }
        } else {
            println!("  [INFO] Could not parse releases");
        }
    } else {
        println!("  [INFO] Could not connect to check for updates");
    }

    println!("\n=== Module Integrity ===");
    if let Ok(modules) = crate::modules::ModuleManager::scan_modules(modules_dir) {
        if modules.is_empty() {
            println!("  [INFO] No modules installed");
        } else {
            println!("  [OK] {} modules found", modules.len());

            for (name, manifest) in &modules {
                let module_path = modules_dir.join(name);
                let mut module_issues = 0;

                if !manifest.name.to_lowercase().contains(&name.to_lowercase()) {
                    println!("  [WARN] Module '{}': name mismatch with folder", name);
                    module_issues += 1;
                }

                let manifest_path = module_path.join("manifest.xml");
                if !manifest_path.exists() {
                    println!("  [WARN] Module '{}': manifest.xml missing", name);
                    module_issues += 1;
                    if !no_fix {
                        let default_manifest = format!(r#"<?xml version="1.0"?>
<module>
    <name>{}</name>
    <executable></executable>
</module>
"#, name);
                        if std::fs::write(&manifest_path, default_manifest).is_ok() {
                            println!("    [FIXED] Created missing manifest.xml for '{}'", name);
                            fixed += 1;
                        }
                        module_issues -= 1;
                    }
                }

                let readme_path = module_path.join("README.md");
                if !readme_path.exists() {
                    if !no_fix {
                        let default_readme = format!("# {}\n\nDescribe what this module does here.\n", name);
                        if std::fs::write(&readme_path, default_readme).is_ok() {
                            println!("    [FIXED] Created missing README.md for '{}'", name);
                            fixed += 1;
                        }
                    } else {
                        println!("  [INFO] Module '{}': README.md missing (run without --no-fix to create)", name);
                    }
                }

                if module_issues > 0 {
                    issues_found += module_issues;
                }
            }
        }
    }

    println!("\n=== Broken Symlinks in Modules ===");
    if let Ok(entries) = std::fs::read_dir(modules_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink() && !path.exists() {
                println!("  [WARN] Broken symlink: {:?}", path);
                issues_found += 1;
                if !no_fix {
                    if std::fs::remove_file(&path).is_ok() {
                        println!("    [FIXED] Removed broken symlink");
                        fixed += 1;
                        issues_found -= 1;
                    }
                }
            }
        }
    }

    println!("\n=== Completions ===");
    let completions_dir = config_dir.join("completions");
    let completion_files = vec![
        completions_dir.join("aktools"),
        completions_dir.join("_aktools"),
        completions_dir.join("aktools.fish"),
    ];
    let mut has_completion = false;
    for cf in &completion_files {
        if cf.exists() {
            has_completion = true;
            break;
        }
    }
    if has_completion {
        println!("  [OK] Shell completions found");
    } else {
        println!("  [INFO] No completions installed. Run 'aktools completion bash --install' to set up");
    }

    if !no_fix && fixed > 0 {
        println!("\n[FIXED] {} issues fixed automatically", fixed);
    }

    if issues_found > 0 {
        if !no_fix {
            println!("\n[ERROR] {} issues found (some auto-fixed).", issues_found);
        } else {
            println!("\n[ERROR] {} issues found. Run without --no-fix to auto-fix.", issues_found);
        }
        1
    } else if !no_fix && fixed == 0 {
        println!("\n[OK] Everything looks good!");
        0
    } else {
        println!("\n[OK] No issues detected");
        0
    }
}

fn detect_scheduler() -> &'static str {
    if std::process::Command::new("which")
        .arg("systemctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "systemd";
    }

    if std::process::Command::new("which")
        .arg("launchctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "launchd";
    }

    if std::process::Command::new("which")
        .arg("crontab")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "cron";
    }

    "unknown"
}

fn check_systemd_autoupdate(_config_dir: &Path, no_fix: bool, fixed: &mut i32, issues_found: &mut i32) {
    let home = dirs::home_dir().unwrap_or_default();
    let service_path = home.join(".config/systemd/user/aktools-updater.service");
    let timer_path = home.join(".config/systemd/user/aktools-updater.timer");

    let mut needs_update = false;

    if service_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&service_path) {
            if content.contains("brew update && brew upgrade aktools") {
                needs_update = true;
                println!("  [WARN] Found deprecated autoupdate service (uses 'brew update')");
                *issues_found += 1;
            } else if content.contains("aktools upgrade") {
                println!("  [OK] Autoupdate service is up to date");
            }
        }
    }

    if needs_update && !no_fix {
        let service_content = r#"[Unit]
Description=AKTools Auto Update

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'aktools upgrade'
"#;

        if std::fs::write(&service_path, service_content).is_ok() {
            println!("    [FIXED] Updated autoupdate service to use 'aktools upgrade'");
            *fixed += 1;
            *issues_found -= 1;

            let _ = std::process::Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .output();
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "enable", "aktools-updater.timer"])
                .output();
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "start", "aktools-updater.timer"])
                .output();
        }
    }

    if !service_path.exists() && !timer_path.exists() {
        println!("  [INFO] No autoupdate timers installed");
    }
}

fn check_launchd_autoupdate(no_fix: bool, fixed: &mut i32, issues_found: &mut i32) {
    let plist_path = dirs::home_dir()
        .map(|h| h.join("Library/LaunchAgents/com.aktools.autoupdate.plist"))
        .unwrap_or_default();

    if plist_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&plist_path) {
            if content.contains("brew update && brew upgrade") {
                println!("  [WARN] Found deprecated autoupdate plist (uses 'brew update')");
                *issues_found += 1;

                if !no_fix {
                    let home = dirs::home_dir().unwrap_or_default();
                    let new_plist = home.join("Library/LaunchAgents/com.aktools.autoupdate.plist");
                    let new_content = content.replace(
                        "brew update && brew upgrade aktools",
                        "aktools upgrade"
                    );
                    if std::fs::write(&new_plist, new_content).is_ok() {
                        println!("    [FIXED] Updated autoupdate plist to use 'aktools upgrade'");
                        *fixed += 1;
                        *issues_found -= 1;

                        let _ = std::process::Command::new("launchctl")
                            .args(["unload", plist_path.to_str().unwrap_or_default()])
                            .output();
                        let _ = std::process::Command::new("launchctl")
                            .args(["load", new_plist.to_str().unwrap_or_default()])
                            .output();
                    }
                }
            } else if content.contains("aktools upgrade") {
                println!("  [OK] Autoupdate plist is up to date");
            }
        }
    } else {
        println!("  [INFO] No autoupdate plist installed");
    }
}

fn check_cron_autoupdate(no_fix: bool, fixed: &mut i32, issues_found: &mut i32) {
    let result = std::process::Command::new("crontab")
        .args(["-l"])
        .output();

    if let Ok(output) = result {
        let crontab = String::from_utf8_lossy(&output.stdout);
        if crontab.contains("aktools") {
            if crontab.contains("brew update && brew upgrade") {
                println!("  [WARN] Found deprecated cron entry (uses 'brew update')");
                *issues_found += 1;

                if !no_fix {
                    let new_crontab = crontab.lines()
                        .map(|line| {
                            if line.contains("brew update && brew upgrade aktools") {
                                line.replace("brew update && brew upgrade aktools", "aktools upgrade")
                            } else {
                                line.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let result = std::process::Command::new("bash")
                        .args(["-c", &format!("echo '{}' | crontab -", new_crontab)])
                        .output();

                    if result.map(|o| o.status.success()).unwrap_or(false) {
                        println!("    [FIXED] Updated cron entry to use 'aktools upgrade'");
                        *fixed += 1;
                        *issues_found -= 1;
                    }
                }
            } else if crontab.contains("aktools upgrade") {
                println!("  [OK] Autoupdate cron entry is up to date");
            }
        } else {
            println!("  [INFO] No aktools cron entry found");
        }
    } else {
        println!("  [INFO] No crontab entries");
    }
}
