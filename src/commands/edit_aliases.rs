use std::path::Path;
use std::fs;
use std::io::{self, Write};

fn parse_aliases(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter(|line| line.trim().starts_with("alias ") && line.contains("='") && line.ends_with("'"))
        .filter_map(|line| {
            let trimmed = line.trim();
            let inner = trimmed.strip_prefix("alias ").unwrap_or(trimmed);
            if let Some(eq_pos) = inner.find("='") {
                let alias_name = inner[..eq_pos].to_string();
                let alias_cmd = inner[eq_pos + 2..inner.len() - 1].to_string();
                Some((alias_name, alias_cmd))
            } else {
                None
            }
        })
        .collect()
}

fn print_aliases(aliases: &[(String, String)]) {
    println!("\nCurrent aliases:");
    if aliases.is_empty() {
        println!("  (none)");
    } else {
        for (name, cmd) in aliases {
            println!("  {:20} -> {}", name, cmd);
        }
    }
    println!();
}

pub fn execute(config_dir: &Path) -> i32 {
    let aliases_file = config_dir.join("aliases.sh");

    println!("Anvil Alias Editor\n");

    let mut current_aliases = if aliases_file.exists() {
        fs::read_to_string(&aliases_file).unwrap_or_default()
    } else {
        println!("Aliases file not found. Creating new one...");
        String::new()
    };

    let mut aliases = parse_aliases(&current_aliases);
    print_aliases(&aliases);

    loop {
        print!("Options: (a)dd alias, (r)emove alias, (q)uit: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Error reading input");
            return 1;
        }

        match input.trim().to_lowercase().chars().next() {
            Some('a') => {
                print!("  Enter alias name: ");
                io::stdout().flush().unwrap();
                let mut alias_name = String::new();
                if io::stdin().read_line(&mut alias_name).is_err() {
                    println!("Error reading input");
                    continue;
                }
                let alias_name = alias_name.trim().to_string();
                if alias_name.is_empty() {
                    println!("  Alias name cannot be empty");
                    continue;
                }
                if aliases.iter().any(|(n, _)| n == &alias_name) {
                    println!("  Alias '{}' already exists", alias_name);
                    continue;
                }

                print!("  Enter command [default: anvil]: ");
                io::stdout().flush().unwrap();
                let mut alias_cmd = String::new();
                if io::stdin().read_line(&mut alias_cmd).is_err() {
                    println!("Error reading input");
                    continue;
                }
                let alias_cmd = alias_cmd.trim();
                let alias_cmd = if alias_cmd.is_empty() {
                    "anvil".to_string()
                } else {
                    format!("anvil {}", alias_cmd)
                };

                let new_alias = format!("alias {}='{}'\n", alias_name, alias_cmd);
                let new_content = if current_aliases.ends_with('\n') {
                    current_aliases.clone() + &new_alias
                } else if current_aliases.is_empty() {
                    new_alias
                } else {
                    current_aliases.clone() + "\n" + &new_alias
                };

                if let Err(e) = fs::write(&aliases_file, &new_content) {
                    println!("  Error writing aliases file: {}", e);
                    continue;
                }

                current_aliases = new_content;
                aliases = parse_aliases(&current_aliases);
                print_aliases(&aliases);
                println!("  Added alias: {} -> {}", alias_name, alias_cmd);
            }
            Some('r') => {
                print!("  Enter alias name to remove: ");
                io::stdout().flush().unwrap();
                let mut alias_name = String::new();
                if io::stdin().read_line(&mut alias_name).is_err() {
                    println!("Error reading input");
                    continue;
                }
                let alias_name = alias_name.trim().to_string();
                if !aliases.iter().any(|(n, _)| n == &alias_name) {
                    println!("  Alias '{}' not found", alias_name);
                    continue;
                }

                let mut new_content: String = current_aliases
                    .lines()
                    .filter(|line| {
                        !line.trim().starts_with(&format!("alias {}", alias_name))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !new_content.is_empty() && !new_content.ends_with('\n') {
                    new_content.push('\n');
                }

                if let Err(e) = fs::write(&aliases_file, new_content.clone()) {
                    println!("  Error writing aliases file: {}", e);
                    continue;
                }

                current_aliases = new_content;
                aliases = parse_aliases(&current_aliases);
                print_aliases(&aliases);
                println!("  Removed alias: {}", alias_name);
            }
            Some('q') => {
                println!("Done.");
                return 0;
            }
            _ => {
                println!("  Invalid option");
            }
        }
    }
}