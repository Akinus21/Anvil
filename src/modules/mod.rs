use std::collections::HashMap;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::os::unix::fs::PermissionsExt;

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

#[derive(Debug, Clone)]
pub struct OptionSwitch {
    pub flags: Vec<String>,
    pub _is_default: bool,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleManifest {
    pub name: String,
    pub aliases: Vec<String>,
    pub executable: String,
    pub options: Vec<OptionSwitch>,
}

pub struct ModuleManager;

impl ModuleManager {
    pub fn load_manifest(module_path: &Path) -> std::io::Result<ModuleManifest> {
        let manifest_path = module_path.join("manifest.xml");
        if !manifest_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("manifest.xml not found in {:?}", module_path),
            ));
        }
        let content = fs::read(&manifest_path)?;
        Self::parse_manifest(&content)
    }

    fn parse_manifest(content: &[u8]) -> std::io::Result<ModuleManifest> {
        let mut reader = Reader::from_reader(content);
        reader.trim_text(true);

        let mut name = String::new();
        let mut aliases = Vec::new();
        let mut executable = String::new();
        let mut options = Vec::new();
        let mut current_option: Option<OptionSwitch> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "name" => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                name = t.unescape().unwrap_or_default().to_string();
                            }
                        }
                        "alias" => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let alias = t.unescape().unwrap_or_default().to_string();
                                aliases.push(alias);
                            }
                        }
                        "executable" => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                executable = t.unescape().unwrap_or_default().to_string();
                            }
                        }
                        "option" => {
                            let is_default = e.attributes().any(|a| {
                                a.map(|attr| {
                                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                    let value = String::from_utf8_lossy(&attr.value).to_string();
                                    key == "default" && value == "true"
                                }).unwrap_or(false)
                            });
                            current_option = Some(OptionSwitch {
                                flags: Vec::new(),
                                _is_default: is_default,
                                commands: Vec::new(),
                            });
                        }
                        "flag" => {
                            if let Some(ref mut opt) = current_option {
                                if let Ok(Event::Text(t)) = reader.read_event() {
                                    let flag = t.unescape().unwrap_or_default().to_string();
                                    opt.flags.push(flag);
                                }
                            }
                        }
                        "command" => {
                            if let Some(ref mut opt) = current_option {
                                if let Ok(Event::Text(t)) = reader.read_event() {
                                    let cmd = t.unescape().unwrap_or_default().to_string();
                                    opt.commands.push(cmd);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "option" {
                        if let Some(opt) = current_option.take() {
                            options.push(opt);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(ModuleManifest {
            name,
            aliases,
            executable,
            options,
        })
    }

    pub fn scan_modules(modules_dir: &Path) -> std::io::Result<HashMap<String, ModuleManifest>> {
        let mut modules = HashMap::new();
        if !modules_dir.exists() {
            return Ok(modules);
        }

        for entry in fs::read_dir(modules_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Ok(manifest) = Self::load_manifest(&path) {
                    modules.insert(manifest.name.clone(), manifest);
                }
            }
        }

        Ok(modules)
    }

    pub fn write_manifest(module_path: &Path, manifest: &ModuleManifest) -> std::io::Result<()> {
        let manifest_path = module_path.join("manifest.xml");

        // Write to temp file first for atomicity
        let temp_path = module_path.join("manifest.xml.tmp");
        let file = fs::File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);

        Self::serialize_manifest(&mut writer, manifest)?;

        // Ensure all data is flushed to disk
        writer.flush()?;

        // Atomic rename (on Unix this is atomic if on same filesystem)
        fs::rename(&temp_path, &manifest_path)?;

        Ok(())
    }

    fn serialize_manifest<W: Write>(writer: &mut W, manifest: &ModuleManifest) -> io::Result<()> {
        let mut xml_writer = Writer::new_with_indent(writer, b' ', 4);

        // XML declaration
        xml_writer.write_event(Event::Decl(quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Root element
        xml_writer.write_event(Event::Start(BytesStart::new("module")))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // name
        xml_writer.write_event(Event::Start(BytesStart::new("name")))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        xml_writer.write_event(Event::Text(BytesText::new(&manifest.name)))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        xml_writer.write_event(Event::End(BytesEnd::new("name")))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // aliases
        for alias in &manifest.aliases {
            xml_writer.write_event(Event::Start(BytesStart::new("alias")))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            xml_writer.write_event(Event::Text(BytesText::new(alias)))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            xml_writer.write_event(Event::End(BytesEnd::new("alias")))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        // executable
        xml_writer.write_event(Event::Start(BytesStart::new("executable")))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        xml_writer.write_event(Event::Text(BytesText::new(&manifest.executable)))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        xml_writer.write_event(Event::End(BytesEnd::new("executable")))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // options
        for opt in &manifest.options {
            let mut opt_start = BytesStart::new("option");
            if opt._is_default {
                opt_start.push_attribute(("default", "true"));
            }
            xml_writer.write_event(Event::Start(opt_start))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            // flags
            for flag in &opt.flags {
                xml_writer.write_event(Event::Start(BytesStart::new("flag")))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                xml_writer.write_event(Event::Text(BytesText::new(flag)))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                xml_writer.write_event(Event::End(BytesEnd::new("flag")))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }

            // commands
            for cmd in &opt.commands {
                xml_writer.write_event(Event::Start(BytesStart::new("command")))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                xml_writer.write_event(Event::Text(BytesText::new(cmd)))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                xml_writer.write_event(Event::End(BytesEnd::new("command")))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }

            xml_writer.write_event(Event::End(BytesEnd::new("option")))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        xml_writer.write_event(Event::End(BytesEnd::new("module")))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }

    pub fn create_module_folder(
        modules_dir: &Path,
        name: &str,
        aliases: &[String],
        source_file: &Path,
        use_link: bool,
    ) -> std::io::Result<std::path::PathBuf> {
        let module_dir = modules_dir.join(name);
        fs::create_dir_all(&module_dir)?;

        let file_name = source_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let dest_file = module_dir.join(&file_name);

        if use_link {
            let absolute_source = source_file.canonicalize().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("Cannot resolve path: {}", e))
            })?;
            std::os::unix::fs::symlink(&absolute_source, &dest_file)?;
        } else {
            fs::copy(source_file, &dest_file)?;
        }

        let manifest = ModuleManifest {
            name: name.to_string(),
            aliases: aliases.to_vec(),
            executable: format!("./{}", file_name),
            options: vec![OptionSwitch {
                flags: Vec::new(),
                _is_default: false,
                commands: Vec::new(),
            }],
        };

        Self::write_manifest(&module_dir, &manifest)?;

        let readme = format!("# {}\n\nDescribe what this module does here.\n", name);
        fs::write(module_dir.join("README.md"), readme)?;

        Ok(module_dir)
    }

    pub fn _write_aliases_to_file(modules_dir: &Path, shell_file: &Path) -> std::io::Result<()> {
        let modules = Self::scan_modules(modules_dir)?;
        let mut content = String::new();

        content.push_str("# aktools module aliases - auto-generated\n");
        content.push_str("# Do not edit manually\n\n");

        for (_, manifest) in &modules {
            if !manifest.executable.is_empty() && manifest.aliases.len() > 1 {
                continue;
            }
            for alias in &manifest.aliases {
                if !manifest.executable.is_empty() {
                    content.push_str(&format!(
                        "alias {}='aktools run {}'\n",
                        alias, manifest.name
                    ));
                } else if let Some(opt) = manifest.options.first() {
                    if let Some(flag) = opt.flags.first() {
                        let clean_flag = flag.trim_start_matches('*');
                        content.push_str(&format!(
                            "alias {}='aktools run {} {}'\n",
                            alias, manifest.name, clean_flag
                        ));
                    }
                }
            }
        }

        if let Some(parent) = shell_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(shell_file, content)
    }

    pub fn has_shell_operators(commands: &[String]) -> bool {
        commands.iter().any(|cmd| {
            cmd.contains("&&")
                || cmd.contains("||")
                || cmd.contains(";")
                || cmd.starts_with("sudo")
                || cmd.contains(" &")
                || cmd.ends_with(" &")
                || cmd.trim_end().ends_with('&')
        })
    }

    pub fn generate_shell_script(module_path: &Path, commands: &[String]) -> std::io::Result<()> {
        let script_path = module_path.join("commands.sh");
        let mut content = String::from("#!/bin/bash\nset -e\n\n");

        for cmd in commands {
            content.push_str(cmd);
            content.push('\n');
        }

        fs::write(&script_path, content)?;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
        Ok(())
    }
}