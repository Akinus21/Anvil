use std::path::Path;
use std::fs;

const DEFAULT_COMMUNITY_REPO: &str = "Akinus21/aktools-modules";

#[derive(serde::Serialize, serde::Deserialize)]
struct RepoConfig {
    repos: Vec<Repo>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Repo {
    user: String,
    repo: String,
    is_default: bool,
}

#[derive(serde::Deserialize)]
struct RegistryJson {
    version: u32,
    modules: Vec<RegistryModule>,
}

#[derive(serde::Deserialize)]
struct RegistryModule {
    id: String,
    name: String,
    version: String,
    author: Option<String>,
    license: Option<String>,
    repository: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
    min_aktools_version: Option<String>,
    last_updated: Option<String>,
}

pub fn execute(config_dir: &Path, args: Vec<String>) -> i32 {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("all");

    match subcommand {
        "aktools" | "self" => upgrade_aktools(),
        "modules" | "mods" => {
            let repos_file = config_dir.join("repos.json");
            let modules_dir = config_dir.join("modules");
            upgrade_modules(&repos_file, &modules_dir)
        }
        "all" | _ => {
            let aktools_result = upgrade_aktools();
            let repos_file = config_dir.join("repos.json");
            let modules_dir = config_dir.join("modules");
            let mod_result = upgrade_modules(&repos_file, &modules_dir);

            if aktools_result == 2 && mod_result == 2 {
                0
            } else {
                aktools_result.max(mod_result)
            }
        }
    }
}

fn upgrade_aktools() -> i32 {
    println!("Checking for AKTools updates...\n");

    let current_version = get_installed_aktools_version().unwrap_or(env!("CARGO_PKG_VERSION"));

    let latest_version = match ureq::get("https://api.github.com/repos/Akinus21/aktools/releases/latest")
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if let Ok(body) = resp.into_string() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    json.get("tag_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim_start_matches('v').to_string())
                        .unwrap_or_else(|| current_version.to_string())
                } else {
                    current_version.to_string()
                }
            } else {
                current_version.to_string()
            }
        }
        Err(_) => current_version.to_string(),
    };

    if current_version == latest_version {
        println!("AKTools is up-to-date! (v{})", current_version);
        return 2;
    }

    println!("Update available: v{} -> v{}", current_version, latest_version);
    println!("Upgrading via Homebrew...\n");

    let update_result = std::process::Command::new("brew")
        .args(["update"])
        .output();

    if let Ok(output) = update_result {
        if !output.status.success() {
            eprintln!("Warning: 'brew update' failed:");
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
    } else {
        eprintln!("Error running brew update: {}", update_result.unwrap_err());
        eprintln!("Make sure Homebrew is installed and in your PATH.");
        return 1;
    }

    let upgrade_result = std::process::Command::new("brew")
        .args(["upgrade", "aktools"])
        .output();

    match upgrade_result {
        Ok(output) => {
            if output.status.success() {
                println!("AKTools upgraded successfully!");
                println!("{}", String::from_utf8_lossy(&output.stdout));
                0
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Not a keyword") || stderr.contains("Cask") {
                    println!("AKTools is a cask. Trying 'brew upgrade --cask aktools'...");
                    if let Ok(cask_output) = std::process::Command::new("brew")
                        .args(["upgrade", "--cask", "aktools"])
                        .output()
                    {
                        if cask_output.status.success() {
                            println!("AKTools upgraded successfully!");
                            println!("{}", String::from_utf8_lossy(&cask_output.stdout));
                            return 0;
                        }
                    }
                }
                eprintln!("Warning: 'brew upgrade aktools' may have failed:");
                eprintln!("{}", stderr);
                1
            }
        }
        Err(e) => {
            eprintln!("Error running brew upgrade: {}", e);
            1
        }
    }
}

fn get_installed_aktools_version() -> Option<String> {
    let output = std::process::Command::new("brew")
        .args(["info", "--json", "aktools"])
        .output()
        .ok()?;

    if output.status.success() {
        if let Ok(text) = String::from_utf8_lossy(&output.stdout).parse::<serde_json::Value>() {
            if let Some(arr) = text.as_array() {
                if let Some(obj) = arr.first() {
                    if let Some(versions) = obj.get("installed").and_then(|v| v.as_array()) {
                        if let Some(v) = versions.first() {
                            if let Some(version) = v.get("version").and_then(|vv| vv.as_str()) {
                                return Some(version.to_string());
                            }
                        }
                    }
                    if let Some(version) = obj.get("versions").and_then(|v| v.as_str()) {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }

    let cellar_path = std::process::Command::new("brew")
        .args(["--prefix"])
        .output()
        .ok()?;

    let cellar = String::from_utf8_lossy(&cellar_path.stdout).trim().to_string();
    let aktools_cellar = format!("{}/Cellar/aktools", cellar);

    if let Ok(entries) = fs::read_dir(&aktools_cellar) {
        if let Some(entry) = entries.filter_map(|e| e.ok()).max_by(|a, b| {
            a.file_name().to_string_lossy().cmp(&b.file_name().to_string_lossy())
        }) {
            return entry.file_name().to_str().map(|s| s.to_string());
        }
    }

    None

fn load_repos_config(repos_file: &Path) -> RepoConfig {
    if let Ok(content) = fs::read_to_string(repos_file) {
        if let Ok(config) = serde_json::from_str(&content) {
            return config;
        }
    }
    RepoConfig { repos: Vec::new() }
}

fn upgrade_modules(repos_file: &Path, modules_dir: &Path) -> i32 {
    println!("\nChecking for module updates...\n");

    let config = load_repos_config(repos_file);

    let repos_to_check: Vec<Repo> = if config.repos.is_empty() {
        vec![Repo {
            user: DEFAULT_COMMUNITY_REPO.split('/').next().unwrap().to_string(),
            repo: DEFAULT_COMMUNITY_REPO.split('/').nth(1).unwrap().to_string(),
            is_default: true,
        }]
    } else {
        config.repos.clone()
    };

    let local_modules: Vec<String> = if modules_dir.exists() {
        if let Ok(entries) = fs::read_dir(modules_dir) {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    if local_modules.is_empty() {
        println!("No modules installed.");
        return 2;
    }

    let mut updated = 0;
    let mut failed: Vec<String> = Vec::new();
    let mut up_to_date = 0;

    for module_name in &local_modules {
        let module_path = modules_dir.join(module_name);
        let manifest_path = module_path.join("manifest.xml");

        for repo in &repos_to_check {
            let registry_url = format!(
                "https://raw.githubusercontent.com/{}/{}/main/registry.json",
                repo.user, repo.repo
            );

            if let Ok(resp) = ureq::get(&registry_url).call() {
                if let Ok(body) = resp.into_string() {
                    if let Ok(registry) = serde_json::from_str::<RegistryJson>(&body) {
                        if let Some(remote_module) = registry.modules.iter().find(|m| m.id.to_lowercase() == module_name.to_lowercase()) {
                            let local_version = if let Ok(local_content) = fs::read_to_string(&manifest_path) {
                                extract_version_from_manifest(&local_content)
                            } else {
                                None
                            };

                            match local_version {
                                Some(lv) if lv != remote_module.version => {
                                    println!("Updating '{}': {} -> {}",
                                        module_name, lv, remote_module.version);

                                    if let Err(e) = download_module(module_name, repo, modules_dir) {
                                        eprintln!("  Failed to update: {}", e);
                                        failed.push(module_name.clone());
                                    } else {
                                        updated += 1;
                                    }
                                }
                                Some(_) => {
                                    up_to_date += 1;
                                }
                                None => {
                                    if fs::read_to_string(&manifest_path).is_ok() {
                                        println!("No version tag in local manifest for '{}', downloading latest...", module_name);
                                        match download_module(module_name, repo, modules_dir) {
                                            Ok(_) => {
                                                updated += 1;
                                                if let Ok(new_content) = fs::read_to_string(&manifest_path) {
                                                    let remote_ver = &remote_module.version;
                                                    if !new_content.contains("<version>") {
                                                        let new_manifest = new_content.replace("<module>", &format!("<module>\n    <version>{}</version>", remote_ver));
                                                        if let Ok(_) = fs::write(&manifest_path, new_manifest) {
                                                            println!("  Added version tag: {}", remote_ver);
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("  Failed to update: {}", e);
                                                failed.push(module_name.clone());
                                            }
                                        }
                                    } else {
                                        println!("Could not read manifest for '{}', downloading...", module_name);
                                        if let Err(e) = download_module(module_name, repo, modules_dir) {
                                            eprintln!("  Failed to update: {}", e);
                                            failed.push(module_name.clone());
                                        } else {
                                            updated += 1;
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    if updated == 0 && failed.is_empty() {
        println!("All modules are up-to-date! ({} modules checked)", local_modules.len());
        return 2;
    }

    if updated > 0 {
        let _ = crate::modules::ModuleManager::_write_aliases_to_file(modules_dir, &modules_dir.parent().unwrap().join("aliases.sh"));
        let _ = crate::commands::update::execute(modules_dir, &modules_dir.parent().unwrap().join("registry.json"));
    }

    println!("\nModule update complete: {} updated, {} failed, {} up-to-date", updated, failed.len(), up_to_date);
    if !failed.is_empty() {
        println!("Failed: {}", failed.join(", "));
    }

    if failed.is_empty() { 0 } else { 1 }
}

fn extract_version_from_manifest(content: &str) -> Option<String> {
    if let Some(start) = content.find("<version>") {
        let start = start + 9;
        if let Some(end) = content.find("</version>") {
            return Some(content[start..end].to_string());
        }
    }
    None
}

fn download_module(module_name: &str, repo: &Repo, modules_dir: &Path) -> Result<(), String> {
    let module_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/{}/manifest.xml",
        repo.user, repo.repo, module_name
    );

    let response = ureq::get(&module_url)
        .call()
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    let manifest_xml = response
        .into_string()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let module_path = modules_dir.join(module_name);
    fs::create_dir_all(&module_path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    fs::write(module_path.join("manifest.xml"), manifest_xml)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(())
}