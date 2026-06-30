use std::path::Path;
use crate::modules::ModuleManager;

pub fn execute(modules_dir: &Path) -> i32 {
    match ModuleManager::scan_modules(modules_dir) {
        Ok(modules) => {
            if modules.is_empty() {
                println!("No modules installed.");
                println!("Run 'anvil add <filename>' to add a module.");
                0
            } else {
                println!("Installed modules ({}):\n", modules.len());
                let mut names: Vec<_> = modules.keys().collect();
                names.sort();
                for name in names {
                    if let Some(manifest) = modules.get(name) {
                        print!("  {} ", name);
                        if !manifest.aliases.is_empty() {
                            print!("[{}]", manifest.aliases.join(", "));
                        }
                        println!();
                    }
                }
                0
            }
        }
        Err(e) => {
            println!("Error scanning modules: {}", e);
            1
        }
    }
}