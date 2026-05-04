use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

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
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("list-repos");
    let module_dir = config_dir.join("modules");
    let repos_file = config_dir.join("repos.json");
    let remaining_args = args[1..].to_vec();

    match subcommand {
        "add-repo" => add_repo(&repos_file, &remaining_args),
        "list-repos" => list_repos(&repos_file),
        "search-mods" | "search-mod" | "mod-search" => search_modules(&repos_file, &remaining_args),
        "install-mods" | "install-mod" | "mod-install" => install_modules(&repos_file, &module_dir, config_dir, &remaining_args),
        "add-mod" => add_mod(&repos_file, &module_dir, config_dir, &remaining_args),
        "update-mod" => update_mod(&repos_file, &module_dir, config_dir, &remaining_args),
        "inspect-mod" => inspect_module(&module_dir, &remaining_args),
        _ => {
            println!("Unknown subcommand: {}", subcommand);
            println!("Usage:");
            println!("  aktools add-repo <user/repo>   Add a repo to track");
            println!("  aktools list-repos             List configured repos");
            println!("  aktools search-mods <term>     Search modules");
            println!("  aktools install-mods <mod> [<mod>...]  Install module(s)");
            println!("  aktools add-mod <module>       Submit module to community repo");
            println!("  aktools update-mod <module>   Update module in a repo (owner only)");
            println!("  aktools inspect-mod <module>  Show module contents");
            1
        }
    }
}

fn add_repo(repos_file: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: aktools add-repo <user/repo>");
        println!("Example: aktools add-repo myname/my-plugins");
        return 1;
    }

    let input = &args[0];
    if !input.contains('/') {
        println!("Invalid repo format. Use: user/repo");
        return 1;
    }

    let parts: Vec<&str> = input.split('/').collect();
    let user = parts[0].to_string();
    let repo = parts[1].to_string();

    let mut config = load_repos_config(repos_file);

    if config.repos.iter().any(|r| r.user == user && r.repo == repo) {
        println!("Repo {}/{} is already tracked", user, repo);
        return 1;
    }

    let test_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/registry.json",
        user, repo
    );

    println!("Checking repo...");
    if let Err(e) = ureq::get(&test_url).call() {
        println!("Error: Could not fetch {}/{} - {}", user, repo, e);
        return 1;
    }

    config.repos.push(Repo {
        user,
        repo,
        is_default: false,
    });

    if let Err(e) = fs::write(repos_file, serde_json::to_string_pretty(&config).unwrap()) {
        println!("Error saving repos config: {}", e);
        return 1;
    }

    println!("Added repo successfully");
    0
}

fn list_repos(repos_file: &Path) -> i32 {
    let config = load_repos_config(repos_file);

    if config.repos.is_empty() {
        println!("No repos configured. Using default community repo:");
        println!("  {} (default)", DEFAULT_COMMUNITY_REPO);
        return 0;
    }

    println!("Configured repos:");
    for repo in &config.repos {
        let mark = if repo.is_default { " [default]" } else { "" };
        println!("  {}/{}{}", repo.user, repo.repo, mark);
    }
    0
}

fn search_modules(repos_file: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: aktools search-mods <term>");
        return 1;
    }

    let query = args.join(" ").to_lowercase();
    let config = load_repos_config(repos_file);
    let mut all_modules: Vec<(String, RegistryModule)> = Vec::new();

    let repos_to_search: Vec<Repo> = if config.repos.is_empty() {
        vec![Repo {
            user: DEFAULT_COMMUNITY_REPO.split('/').next().unwrap().to_string(),
            repo: DEFAULT_COMMUNITY_REPO.split('/').nth(1).unwrap().to_string(),
            is_default: true,
        }]
    } else {
        config.repos
    };

    println!("Searching for: {}", query);
    for repo in repos_to_search {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/main/registry.json",
            repo.user, repo.repo
        );

        match ureq::get(&url).call() {
            Ok(response) => {
                if let Ok(body) = response.into_string() {
                    if let Ok(registry) = serde_json::from_str::<RegistryJson>(&body) {
                        for module in registry.modules {
                            if module.id.to_lowercase().contains(&query)
                                || module.name.to_lowercase().contains(&query)
                                || module.description.as_ref().map(|d| d.to_lowercase().contains(&query)).unwrap_or(false)
                                || module.tags.as_ref().map(|t| t.iter().any(|tag| tag.to_lowercase().contains(&query))).unwrap_or(false)
                            {
                                all_modules.push((format!("{}/{}", repo.user, repo.repo), module));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Warning: Could not fetch {}: {}", repo.repo, e);
            }
        }
    }

    if all_modules.is_empty() {
        println!("No modules found matching: {}", query);
    } else {
        println!("\nFound {} module(s):", all_modules.len());
        for (repo_name, module) in all_modules {
            println!("\n  [{}]", repo_name);
            println!("  {:20} ({})", module.name, module.id);
            if let Some(desc) = &module.description {
                println!("  {}", desc);
            }
            if let Some(tags) = &module.tags {
                if !tags.is_empty() {
                    println!("  Tags: {}", tags.join(", "));
                }
            }
        }
    }
    0
}

fn install_modules(repos_file: &Path, modules_dir: &Path, config_dir: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: aktools install-mods <module> [<module>...]");
        return 1;
    }

    let module_names: Vec<String> = args.iter().map(|s| s.to_lowercase()).collect();
    let config = load_repos_config(repos_file);

    let repos_to_search: Vec<Repo> = if config.repos.is_empty() {
        vec![Repo {
            user: DEFAULT_COMMUNITY_REPO.split('/').next().unwrap().to_string(),
            repo: DEFAULT_COMMUNITY_REPO.split('/').nth(1).unwrap().to_string(),
            is_default: true,
        }]
    } else {
        config.repos
    };

    let mut installed = 0;
    let mut failed: Vec<String> = Vec::new();

    for module_name in &module_names {
        println!("\nInstalling: {}", module_name);
        let mut found = false;

        for repo in &repos_to_search {
            let registry_url = format!(
                "https://raw.githubusercontent.com/{}/{}/main/registry.json",
                repo.user, repo.repo
            );

            let module_url = format!(
                "https://raw.githubusercontent.com/{}/{}/main/{}/manifest.xml",
                repo.user, repo.repo, module_name
            );

            match ureq::get(&registry_url).call() {
                Ok(response) => {
                    if let Ok(body) = response.into_string() {
                        if let Ok(registry) = serde_json::from_str::<RegistryJson>(&body) {
                            if registry.modules.iter().any(|m| m.id.to_lowercase() == *module_name) {
                                match ureq::get(&module_url).call() {
                                    Ok(xml_response) => {
                                        if let Ok(manifest_xml) = xml_response.into_string() {
                                            let target_dir = modules_dir.join(module_name);
                                            if target_dir.exists() {
                                                println!("  Module already exists, skipping");
                                                found = true;
                                            } else {
                                                match fs::create_dir_all(&target_dir) {
                                                    Ok(_) => {
                                                        let manifest_path = target_dir.join("manifest.xml");
                                                        if fs::write(&manifest_path, manifest_xml).is_ok() {
                                                            println!("  Installed from {}/{}", repo.user, repo.repo);
                                                            found = true;
                                                            installed += 1;
                                                        } else {
                                                            println!("  Error: Could not write manifest");
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!("  Error: Could not create module directory: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }
                }
                Err(_) => {}
            }

            if found {
                break;
            }
        }

        if !found {
            println!("  Error: Module '{}' not found in any repo", module_name);
            failed.push(module_name.clone());
        }
    }

    println!("\nInstalled {}/{} module(s)", installed, module_names.len());
    if !failed.is_empty() {
        println!("Failed: {}", failed.join(", "));
    }

    if installed > 0 {
        let _ = crate::modules::ModuleManager::_write_aliases_to_file(modules_dir, &config_dir.join("aliases.sh"));
        let _ = crate::commands::update::execute(modules_dir, &config_dir.join("registry.json"));
    }

    if failed.is_empty() {
        0
    } else {
        1
    }
}

fn load_repos_config(repos_file: &Path) -> RepoConfig {
    if let Ok(content) = fs::read_to_string(repos_file) {
        if let Ok(config) = serde_json::from_str(&content) {
            return config;
        }
    }
    RepoConfig { repos: Vec::new() }
}

fn add_mod(repos_file: &Path, modules_dir: &Path, _config_dir: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: aktools add-mod <module-name>");
        println!("Submit a local module to the community repo for review.");
        println!("This will fork the repo, add your module, and create a pull request.");
        return 1;
    }

    let module_name = &args[0];

    if module_name.contains('/') || module_name.contains('\\') || module_name.contains(' ') {
        println!("Error: Invalid module name. Cannot contain '/', '\\', or spaces.");
        return 1;
    }

    if module_name.is_empty() || module_name.len() > 50 {
        println!("Error: Module name must be 1-50 characters.");
        return 1;
    }

    let module_path = modules_dir.join(module_name);

    if !module_path.exists() {
        println!("Error: Module '{}' not found in ~/.aktools/modules/", module_name);
        return 1;
    }

    let manifest_path = module_path.join("manifest.xml");
    if !manifest_path.exists() {
        println!("Error: Module '{}' has no manifest.xml", module_name);
        return 1;
    }

    let manifest_content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Error reading manifest: {}", e);
            return 1;
        }
    };

    let gh_token = if let Ok(token) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if token.status.success() {
            String::from_utf8_lossy(&token.stdout).trim().to_string()
        } else {
            std::env::var("GH_TOKEN")
                .or_else(|_| std::env::var("GITHUB_TOKEN"))
                .unwrap_or_default()
        }
    } else {
        std::env::var("GH_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .unwrap_or_default()
    };

    if gh_token.is_empty() {
        println!("Error: Not authenticated with GitHub.");
        println!("Run 'gh auth login' or set GH_TOKEN environment variable.");
        return 1;
    }

    let repos_config = load_repos_config(repos_file);

    let mut repos: Vec<(&str, &str)> = vec![("Akinus21", "aktools-modules")];
    for repo in &repos_config.repos {
        if repo.user != "Akinus21" || repo.repo != "aktools-modules" {
            repos.push((repo.user.as_str(), repo.repo.as_str()));
        }
    }

    println!("\nSelect a repo to submit to:");
    for (i, (user, repo)) in repos.iter().enumerate() {
        let mark = if i == 0 { " (default)" } else { "" };
        println!("  {} - {}/{}{}", i + 1, user, repo, mark);
    }

    print!("Enter number (1-{}): ", repos.len());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        println!("Error reading input");
        return 1;
    }

    let selection = match input.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= repos.len() => n - 1,
        _ => {
            println!("Invalid selection, using default (1).");
            0
        }
    };

    let (repo_owner, repo_name) = repos[selection];
    let api_base = "https://api.github.com";

    println!("\nValidating repository access...");
    let client = ureq::Agent::new();

    match client.get(&format!("{}/repos/{}/{}", api_base, repo_owner, repo_name))
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if resp.status() != 200 {
                println!("Error: Cannot access repository {}/{} (status {})", repo_owner, repo_name, resp.status());
                return 1;
            }
        }
        Err(e) => {
            println!("Error: Cannot access repository {}/{}: {}", repo_owner, repo_name, e);
            return 1;
        }
    }

    let user_login: String = match client.get(&format!("{}/user", api_base))
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if let Ok(body) = resp.into_string() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    json.get("login").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    };

    println!("Checking if module '{}' exists in registry...", module_name);
    let registry_check_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/registry.json",
        repo_owner, repo_name
    );

    match ureq::get(&registry_check_url).call() {
        Ok(resp) => {
            if let Ok(body) = resp.into_string() {
                if let Ok(registry) = serde_json::from_str::<RegistryJson>(&body) {
                    if registry.modules.iter().any(|m| m.id.to_lowercase() == module_name.to_lowercase()) {
                        println!("Warning: Module '{}' already exists in {}/{}", module_name, repo_owner, repo_name);
                        println!("A PR to update it will still be created if you continue.");
                        print!("Continue? (y/N): ");
                        std::io::stdout().flush().unwrap();
                        let mut confirm = String::new();
                        if std::io::stdin().read_line(&mut confirm).is_err() || confirm.trim().to_lowercase() != "y" {
                            println!("Cancelled.");
                            return 0;
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }

    println!("\nSubmitting '{}' to {}/{}...", module_name, repo_owner, repo_name);

    let fork_url = format!("{}/repos/{}/{}/forks", api_base, repo_owner, repo_name);
    println!("Forking repository...");

    let fork_response = client.post(&fork_url)
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .send_string("{}");

    let fork_full_name = match fork_response {
        Ok(resp) => {
            if resp.status() == 202 || resp.status() == 201 {
                if let Ok(body) = resp.into_string() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        json.get("full_name").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if resp.status() == 202 {
                let body = resp.into_string().unwrap_or_default();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    json.get("full_name").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                eprintln!("Error: Could not fork repo (status {})", resp.status());
                None
            }
        }
        Err(e) => {
            eprintln!("Error forking repository: {}", e);
            None
        }
    };

    let fork_full_name = match fork_full_name {
        Some(name) => name,
        None => {
            if !user_login.is_empty() {
                let existing_fork_name = format!("{}/{}", user_login, repo_name);
                println!("Fork may still be creating or already exists. Using: {}", existing_fork_name);
                existing_fork_name
            } else {
                println!("Error: Could not determine fork name. Please try again later.");
                return 1;
            }
        }
    };

    println!("Waiting for fork to be ready...");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let actual_fork_name = fork_full_name.as_str();

    println!("Creating module files in your fork...");

    let files: Vec<PathBuf> = collect_files_recursive(&module_path, &module_path)
        .into_iter()
        .filter(|p| {
            p.ends_with("manifest.xml") || p.ends_with(".sh") || p.ends_with(".bash") || p.ends_with(".py") || p.ends_with(".pl") || p.ends_with(".rb")
        })
        .collect();

    for file_path in &files {
        let relative_path = file_path.strip_prefix(&module_path).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| file_path.to_string_lossy().to_string());
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading {}: {}", file_path.display(), e);
                continue;
            }
        };

        let encoded_content = base64_encode(&content);
        let file_path_api = format!("{}/repos/{}/contents/{}", api_base, actual_fork_name, format!("{}/{}", module_name, relative_path));

        let sha = get_file_sha(&client, &file_path_api, &gh_token);

        let mut file_body = serde_json::json!({
            "message": format!("Add {} file from aktools add-mod", relative_path),
            "content": encoded_content
        });
        if let Some(ref s) = sha {
            file_body["sha"] = serde_json::json!(s);
        }

        let file_response = client.put(&file_path_api)
            .set("Authorization", &format!("Bearer {}", gh_token))
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", "2022-11-28")
            .set("Content-Type", "application/json")
            .send_string(&serde_json::to_string(&file_body).unwrap());

        match file_response {
            Ok(resp) => {
                let status = resp.status();
                if status == 201 {
                    println!("  Added: {}", relative_path);
                } else if status == 200 {
                    println!("  Updated: {}", relative_path);
                } else {
                    let err_body = resp.into_string().unwrap_or_default();
                    eprintln!("  Warning: {} returned status {}: {}", relative_path, status, err_body);
                }
            }
            Err(e) => {
                eprintln!("  Error adding {}: {}", relative_path, e);
            }
        }
    }

    let registry_url = format!("{}/repos/{}/contents/registry.json", api_base, actual_fork_name);

    let registry_sha = match client.get(&registry_url)
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if resp.status() == 200 {
                if let Ok(body) = resp.into_string() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        json.get("sha").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let _base_registry: serde_json::Value = serde_json::from_str(r#"{"version":1,"modules":[]}"#).unwrap();
    let (updated_registry, sha_to_use) = if let Some(ref sha) = registry_sha {
        let current_registry: serde_json::Value = serde_json::from_str(r#"{"version":1,"modules":[]}"#).unwrap();

        let new_module = serde_json::json!({
            "id": module_name,
            "name": module_name,
            "version": "1.0.0",
            "author": if user_login.is_empty() { "unknown" } else { &user_login },
            "description": format!("Module submitted via aktools add-mod"),
            "tags": []
        });

        let mut modules = current_registry.get("modules")
            .and_then(|v| v.as_array())
            .map(|arr| arr.clone())
            .unwrap_or_default();

        modules.retain(|m| m.get("id").and_then(|v| v.as_str()) != Some(module_name.as_str()));
        modules.push(new_module);

        let registry = serde_json::json!({
            "version": current_registry.get("version").unwrap_or(&serde_json::json!(1)),
            "modules": modules
        });
        (registry, Some(sha.clone()))
    } else {
        let new_module = serde_json::json!({
            "id": module_name,
            "name": module_name,
            "version": "1.0.0",
            "author": if user_login.is_empty() { "unknown" } else { &user_login },
            "description": format!("Module submitted via aktools add-mod"),
            "tags": []
        });

        let registry = serde_json::json!({
            "version": 1,
            "modules": [new_module]
        });
        (registry, None)
    };

    let registry_content = serde_json::to_string_pretty(&updated_registry).unwrap();
    let encoded_registry = base64_encode(&registry_content);

    let registry_body = serde_json::json!({
        "message": format!("Update registry.json to add {} module", module_name),
        "content": encoded_registry,
        "sha": sha_to_use
    });

    let registry_response = client.put(&registry_url)
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("Content-Type", "application/json")
        .send_string(&serde_json::to_string(&registry_body).unwrap());

    match registry_response {
        Ok(resp) => {
            if resp.status() == 200 || resp.status() == 201 {
                println!("Updated registry.json");
            } else {
                println!("Warning: registry.json update returned status {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Error updating registry.json: {}", e);
        }
    }

    let pr_url = format!("{}/repos/{}/{}/pulls", api_base, repo_owner, repo_name);
    let head_branch = format!("add-{}", module_name);

    let pr_body = serde_json::json!({
        "title": format!("feat: add {} module", module_name),
        "head": format!("{}:{}", actual_fork_name.split('/').next().unwrap_or(""), head_branch),
        "base": "main",
        "body": format!(
            "## Module: {}\n\n\
            Submitted via `aktools add-mod {}`\n\n\
            ### manifest.xml\n\
            ```xml\n{}\n```",
            module_name, module_name, manifest_content
        )
    });

    println!("Creating pull request...");
    println!("  Head: {}:{}", actual_fork_name.split('/').next().unwrap_or(""), head_branch);
    println!("  Base: main");

    let pr_response = client.post(&pr_url)
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("Content-Type", "application/json")
        .send_string(&serde_json::to_string(&pr_body).unwrap());

    match pr_response {
        Ok(resp) => {
            let status = resp.status();
            if status == 201 {
                if let Ok(pr_body_resp) = resp.into_string() {
                    if let Ok(pr) = serde_json::from_str::<serde_json::Value>(&pr_body_resp) {
                        if let Some(html_url) = pr.get("html_url").and_then(|v| v.as_str()) {
                            println!("\nSuccess! Pull request created: {}", html_url);
                            println!("\nWhen the PR is merged, the module will be added to the registry.");
                            return 0;
                        }
                    }
                }
                println!("\nSuccess! Pull request created.");
                0
            } else {
                let err_body = resp.into_string().unwrap_or_default();
                eprintln!("Error: PR creation returned status {}: {}", status, err_body);

                if err_body.contains("head") && err_body.contains("not found") {
                    eprintln!("\nNote: The fork may still be creating. Try again in a few seconds.");
                    eprintln!("Or check your fork at: https://github.com/{}/aktools-modules", actual_fork_name.split('/').next().unwrap_or(""));
                }
                1
            }
        }
        Err(e) => {
            eprintln!("Error creating pull request: {}", e);
            1
        }
    }
}

fn base64_encode(input: &str) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    let bytes = input.as_bytes();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        output.push(alphabet[b0 >> 2] as char);
        output.push(alphabet[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            output.push(alphabet[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            output.push('=');
        }

        if chunk.len() > 2 {
            output.push(alphabet[b2 & 0x3f] as char);
        } else {
            output.push('=');
        }
    }

    output
}

fn collect_files_recursive(dir: &Path, base: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_files_recursive(&path, base));
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn get_file_sha(client: &ureq::Agent, file_url: &str, token: &str) -> Option<String> {
    match client.get(file_url)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if resp.status() == 200 {
                if let Ok(body) = resp.into_string() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        return json.get("sha").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

fn inspect_module(modules_dir: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: aktools inspect-mod <module-name>");
        println!("Display contents of an installed module.");
        return 1;
    }

    let module_name = &args[0];
    let module_path = modules_dir.join(module_name);

    if !module_path.exists() {
        println!("Error: Module '{}' not found in ~/.aktools/modules/", module_name);
        return 1;
    }

    if !module_path.is_dir() {
        println!("Error: '{}' is not a directory", module_name);
        return 1;
    }

    println!("Module: {}", module_name);
    println!("Path: {}\n", module_path.display());
    println!("Contents:");

    match std::fs::read_dir(&module_path) {
        Ok(entries) => {
            let mut items: Vec<_> = entries.flatten().collect();
            items.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            for entry in items {
                let file_name = entry.file_name();
                let file_type = if entry.path().is_dir() {
                    "/"
                } else if entry.path().is_symlink() {
                    "@"
                } else {
                    ""
                };
                println!("  {}{}", file_name.to_string_lossy(), file_type);
            }
        }
        Err(e) => {
            println!("Error reading module directory: {}", e);
            return 1;
        }
    }

    println!("\nTo work in this directory, run:");
    println!("  cd {}", module_path.display());
    0
}

fn update_mod(_repos_file: &Path, modules_dir: &Path, _config_dir: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: aktools update-mod <module-name> [user/repo]");
        println!("Update a module you own in a repo.");
        println!("If no repo specified, uses Akinus21/aktools-modules");
        return 1;
    }

    let module_name = &args[0];
    let (repo_owner, repo_name) = if args.len() > 1 && args[1].contains('/') {
        let parts: Vec<&str> = args[1].split('/').collect();
        (parts[0].to_string(), parts[1].to_string())
    } else {
        ("Akinus21".to_string(), "aktools-modules".to_string())
    };

    let module_path = modules_dir.join(module_name);
    if !module_path.exists() {
        println!("Error: Module '{}' not found in ~/.aktools/modules/", module_name);
        return 1;
    }

    let manifest_path = module_path.join("manifest.xml");
    if !manifest_path.exists() {
        println!("Error: Module '{}' has no manifest.xml", module_name);
        return 1;
    }

    let _manifest_content = match fs::read_to_string(&manifest_path) {
        Ok(_c) => _c,
        Err(e) => {
            println!("Error reading manifest: {}", e);
            return 1;
        }
    };

    let gh_token = if let Ok(token) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if token.status.success() {
            String::from_utf8_lossy(&token.stdout).trim().to_string()
        } else {
            std::env::var("GH_TOKEN")
                .or_else(|_| std::env::var("GITHUB_TOKEN"))
                .unwrap_or_default()
        }
    } else {
        std::env::var("GH_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .unwrap_or_default()
    };

    if gh_token.is_empty() {
        println!("Error: Not authenticated with GitHub.");
        println!("Run 'gh auth login' or set GH_TOKEN environment variable.");
        return 1;
    }

    let api_base = "https://api.github.com";
    let client = ureq::Agent::new();

    println!("Checking repo access...");
    match client.get(&format!("{}/repos/{}/{}", api_base, repo_owner, repo_name))
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if resp.status() != 200 {
                println!("Error: Cannot access repository {}/{} (status {})", repo_owner, repo_name, resp.status());
                return 1;
            }
        }
        Err(e) => {
            println!("Error: Cannot access repository {}/{}: {}", repo_owner, repo_name, e);
            return 1;
        }
    }

    let user_login: String = match client.get(&format!("{}/user", api_base))
        .set("Authorization", &format!("Bearer {}", gh_token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
    {
        Ok(resp) => {
            if let Ok(body) = resp.into_string() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    json.get("login").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    };

    let has_push_access = user_login == repo_owner;

    if !has_push_access {
        println!("Error: You don't have push access to {}/{}", repo_owner, repo_name);
        println!("Update-mod is only for repo owners. Use 'aktools add-mod' to submit a PR.");
        return 1;
    }

    println!("Updating '{}' in {}/{}...", module_name, repo_owner, repo_name);

    let files: Vec<PathBuf> = collect_files_recursive(&module_path, &module_path)
        .into_iter()
        .filter(|p| {
            p.ends_with("manifest.xml") || p.ends_with(".sh") || p.ends_with(".bash") || p.ends_with(".py") || p.ends_with(".pl") || p.ends_with(".rb")
        })
        .collect();

    for file_path in &files {
        let relative_path = file_path.strip_prefix(&module_path).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| file_path.to_string_lossy().to_string());
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading {}: {}", file_path.display(), e);
                continue;
            }
        };

        let encoded_content = base64_encode(&content);
        let file_path_api = format!("{}/repos/{}/{}/contents/{}", api_base, repo_owner, repo_name, format!("{}/{}", module_name, relative_path));

        let sha = get_file_sha(&client, &file_path_api, &gh_token);

        let mut file_body = serde_json::json!({
            "message": format!("Update {} file via aktools update-mod", relative_path),
            "content": encoded_content
        });
        if let Some(ref s) = sha {
            file_body["sha"] = serde_json::json!(s);
        }

        let file_response = client.put(&file_path_api)
            .set("Authorization", &format!("Bearer {}", gh_token))
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", "2022-11-28")
            .set("Content-Type", "application/json")
            .send_string(&serde_json::to_string(&file_body).unwrap());

        match file_response {
            Ok(resp) => {
                let status = resp.status();
                if status == 201 {
                    println!("  Added: {}", relative_path);
                } else if status == 200 {
                    println!("  Updated: {}", relative_path);
                } else {
                    let err_body = resp.into_string().unwrap_or_default();
                    eprintln!("  Warning: {} returned status {}: {}", relative_path, status, err_body);
                }
            }
            Err(e) => {
                eprintln!("  Error updating {}: {}", relative_path, e);
            }
        }
    }

    println!("\nModule '{}' updated successfully in {}/{}", module_name, repo_owner, repo_name);
    0
}