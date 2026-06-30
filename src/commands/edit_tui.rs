use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::modules::{ModuleManager, ModuleManifest, OptionSwitch};

/// Represents an editable field in the form
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Name,
    Aliases,
    Executable,
    Options(usize),           // option index
    OptionFlag(usize, usize), // option index, flag index
    OptionCommand(usize, usize), // option index, command index
}

/// Application state
pub struct AppState {
    pub manifest: ModuleManifest,
    pub dirty: bool,
    pub selected_field: Field,
    pub editing: bool,
    pub input_buffer: String,
    pub message: String,
    pub module_path: PathBuf,
}

impl AppState {
    pub fn new(manifest: ModuleManifest, module_path: PathBuf) -> Self {
        Self {
            manifest,
            dirty: false,
            selected_field: Field::Name,
            editing: false,
            input_buffer: String::new(),
            message: String::new(),
            module_path,
        }
    }

    /// Get all navigable fields in order
    pub fn get_fields(&self) -> Vec<Field> {
        let mut fields = vec![Field::Name, Field::Aliases, Field::Executable];
        
        for opt_idx in 0..self.manifest.options.len() {
            fields.push(Field::Options(opt_idx));
            let opt = &self.manifest.options[opt_idx];
            for flag_idx in 0..opt.flags.len() {
                fields.push(Field::OptionFlag(opt_idx, flag_idx));
            }
            for cmd_idx in 0..opt.commands.len() {
                fields.push(Field::OptionCommand(opt_idx, cmd_idx));
            }
        }
        
        fields
    }

    /// Move to next field
    pub fn next_field(&mut self) {
        let fields = self.get_fields();
        if fields.is_empty() {
            return;
        }
        if let Some(current_idx) = fields.iter().position(|f| f == &self.selected_field) {
            if current_idx + 1 < fields.len() {
                self.selected_field = fields[current_idx + 1].clone();
                self.editing = false;
                self.input_buffer.clear();
            }
        }
    }

    /// Move to previous field
    pub fn prev_field(&mut self) {
        let fields = self.get_fields();
        if fields.is_empty() {
            return;
        }
        if let Some(current_idx) = fields.iter().position(|f| f == &self.selected_field) {
            if current_idx > 0 {
                self.selected_field = fields[current_idx - 1].clone();
                self.editing = false;
                self.input_buffer.clear();
            }
        }
    }

    /// Get display label for current field
    pub fn get_field_label(&self) -> String {
        match &self.selected_field {
            Field::Name => "Name".to_string(),
            Field::Aliases => "Aliases".to_string(),
            Field::Executable => "Executable".to_string(),
            Field::Options(idx) => format!("Option {}", idx + 1),
            Field::OptionFlag(opt_idx, flag_idx) => {
                format!("Option {} Flag {}", opt_idx + 1, flag_idx + 1)
            }
            Field::OptionCommand(opt_idx, cmd_idx) => {
                format!("Option {} Command {}", opt_idx + 1, cmd_idx + 1)
            }
        }
    }

    /// Get current value for display
    pub fn get_current_value(&self) -> String {
        match &self.selected_field {
            Field::Name => self.manifest.name.clone(),
            Field::Aliases => self.manifest.aliases.join(", "),
            Field::Executable => {
                if self.manifest.executable.is_empty() {
                    "(none - command-only)".to_string()
                } else {
                    self.manifest.executable.clone()
                }
            }
            Field::Options(idx) => {
                if *idx < self.manifest.options.len() {
                    let opt = &self.manifest.options[*idx];
                    format!("{} flag(s), {} command(s)", opt.flags.len(), opt.commands.len())
                } else {
                    "(none)".to_string()
                }
            }
            Field::OptionFlag(opt_idx, flag_idx) => {
                if *opt_idx < self.manifest.options.len() {
                    let opt = &self.manifest.options[*opt_idx];
                    if *flag_idx < opt.flags.len() {
                        opt.flags[*flag_idx].clone()
                    } else {
                        "(none)".to_string()
                    }
                } else {
                    "(none)".to_string()
                }
            }
            Field::OptionCommand(opt_idx, cmd_idx) => {
                if *opt_idx < self.manifest.options.len() {
                    let opt = &self.manifest.options[*opt_idx];
                    if *cmd_idx < opt.commands.len() {
                        opt.commands[*cmd_idx].clone()
                    } else {
                        "(none)".to_string()
                    }
                } else {
                    "(none)".to_string()
                }
            }
        }
    }

    /// Apply the current input to the manifest
    pub fn apply_edit(&mut self) {
        let value = self.input_buffer.trim().to_string();
        
        match &self.selected_field {
            Field::Name => {
                if !value.is_empty() && value != self.manifest.name {
                    self.manifest.name = value;
                    self.dirty = true;
                }
            }
            Field::Aliases => {
                let aliases: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.manifest.aliases = aliases;
                self.dirty = true;
            }
            Field::Executable => {
                self.manifest.executable = value;
                self.dirty = true;
            }
            Field::OptionFlag(opt_idx, flag_idx) => {
                if *opt_idx < self.manifest.options.len() {
                    let flags = &self.manifest.options[*opt_idx].flags;
                    if *flag_idx < flags.len() && !value.is_empty() {
                        self.manifest.options[*opt_idx].flags[*flag_idx] = value;
                        self.dirty = true;
                    }
                }
            }
            Field::OptionCommand(opt_idx, cmd_idx) => {
                if *opt_idx < self.manifest.options.len() {
                    let commands = &self.manifest.options[*opt_idx].commands;
                    if *cmd_idx < commands.len() && !value.is_empty() {
                        self.manifest.options[*opt_idx].commands[*cmd_idx] = value;
                        self.dirty = true;
                    }
                }
            }
            _ => {}
        }
        
        self.editing = false;
        self.input_buffer.clear();
    }

    /// Generate live XML preview
    pub fn generate_preview(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\"?>\n");
        xml.push_str("<module>\n");
        
        // Name
        xml.push_str(&format!("    <name>{}</name>\n", self.escape_xml(&self.manifest.name)));
        
        // Aliases
        for alias in &self.manifest.aliases {
            xml.push_str(&format!("    <alias>{}</alias>\n", self.escape_xml(alias)));
        }
        
        // Executable
        xml.push_str(&format!("    <executable>{}</executable>\n", self.escape_xml(&self.manifest.executable)));
        
        // Options
        for opt in &self.manifest.options {
            if opt._is_default {
                xml.push_str("    <option default=\"true\">\n");
            } else {
                xml.push_str("    <option>\n");
            }
            
            for flag in &opt.flags {
                xml.push_str(&format!("        <flag>{}</flag>\n", self.escape_xml(flag)));
            }
            
            for cmd in &opt.commands {
                xml.push_str(&format!("        <command>{}</command>\n", self.escape_xml(cmd)));
            }
            
            xml.push_str("    </option>\n");
        }
        
        xml.push_str("</module>");
        xml
    }

    fn escape_xml(&self, s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Save the manifest
    pub fn save(&mut self) -> Result<(), String> {
        ModuleManager::write_manifest(&self.module_path, &self.manifest)
            .map_err(|e| format!("Save failed: {}", e))?;
        self.dirty = false;
        self.message = "Saved successfully!".to_string();
        Ok(())
    }
}

/// Run the TUI editor
pub fn run(modules_dir: &Path, module_name: Option<String>) -> i32 {
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
        match select_module_tui(&modules) {
            Some(name) => name,
            None => return 0,
        }
    };

    // Phase 3: Load the manifest
    let module_path = modules_dir.join(&selected_name);
    let manifest = match ModuleManager::load_manifest(&module_path) {
        Ok(m) => m,
        Err(e) => {
            println!("Error loading module manifest: {}", e);
            return 1;
        }
    };

    // Run the TUI
    if let Err(e) = run_tui(manifest, module_path) {
        println!("TUI error: {}", e);
        return 1;
    }

    0
}

/// Simple module selection TUI
fn select_module_tui(modules: &std::collections::HashMap<String, ModuleManifest>) -> Option<String> {
    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}", e);
        return None;
    }
    let terminal = match Terminal::new(CrosstermBackend::new(std::io::stdout())) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create terminal: {}", e);
            let _ = disable_raw_mode();
            return None;
        }
    };
    let mut terminal = terminal;
    
    let mut selected = 0;
    let mut names: Vec<String> = modules.keys().cloned().collect();
    names.sort();
    
    loop {
        if terminal.draw(|f| {
            let area = f.size();
            
            // Title
            let title = Paragraph::new("Select Module to Edit")
                .style(Style::default().fg(Color::Yellow).bold());
            f.render_widget(title, Rect::new(area.x, area.y, area.width, 1));
            
            // Module list
            let items: Vec<ListItem> = names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let style = if i == selected {
                        Style::default().fg(Color::Black).bg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(Span::styled(name, style)))
                })
                .collect();
            
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Modules"))
                .start_corner(ratatui::layout::Corner::TopLeft);
            
            f.render_widget(list, Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(4)));
            
            // Help
            let help = Paragraph::new("↑/↓: Select  Enter: Edit  Esc: Quit");
            f.render_widget(help, Rect::new(area.x, area.y + area.height.saturating_sub(2), area.width, 1));
        }).is_err() {
            eprintln!("Draw error");
            thread::sleep(Duration::from_millis(50));
            continue;
        };
        
        let event = match event::read() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Event read error: {}", e);
                continue;
            }
        };
        
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected < names.len().saturating_sub(1) {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    disable_raw_mode().ok();
                    return Some(names[selected].clone());
                }
                KeyCode::Esc => {
                    disable_raw_mode().ok();
                    return None;
                }
                _ => {}
            }
        }
    }
}

/// Main TUI event loop
fn run_tui(manifest: ModuleManifest, module_path: std::path::PathBuf) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;

    // Guard to ensure raw mode is disabled on scope exit
    struct RawModeGuard;
    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
        }
    }
    let _raw_guard = RawModeGuard;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| format!("Failed to enter alternate screen: {}", e))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;

    let mut state = AppState::new(manifest, module_path);
    let mut quit_confirm = false;

    let result = run_event_loop(&mut terminal, &mut state, &mut quit_confirm);

    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| format!("Failed to leave alternate screen: {}", e))?;
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;

    result
}

/// Event loop for the TUI
fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    quit_confirm: &mut bool,
) -> Result<(), String> {
    loop {
        terminal.draw(|f| {
            let area = f.size();
            
            // Split into left (form) and right (preview) panels
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(area);
            
            // Left panel: Form
            render_form(f, chunks[0], state);
            
            // Right panel: XML Preview
            render_preview(f, chunks[1], state);
            
            // Bottom bar: Help and status
            render_bottom_bar(f, Rect::new(area.x, area.y + area.height.saturating_sub(2), area.width, 2), state);
        }).map_err(|e| format!("Draw error: {}", e))?;
        
        // Handle input
        if let Event::Key(key) = event::read().map_err(|e| format!("Event read error: {}", e))? {
            
            // If we're editing, handle input mode
            if state.editing {
                match key.code {
                    KeyCode::Enter => {
                        state.apply_edit();
                    }
                    KeyCode::Esc => {
                        state.editing = false;
                        state.input_buffer.clear();
                    }
                    KeyCode::Backspace => {
                        state.input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        state.input_buffer.push(c);
                    }
                    _ => {}
                }
                continue;
            }
            
            // Navigation mode
            match key.code {
                KeyCode::Tab => {
                    *quit_confirm = false;
                    if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                        state.prev_field();
                    } else {
                        state.next_field();
                    }
                }
                KeyCode::Enter => {
                    // Start editing (except for Options which is informational)
                    match state.selected_field {
                        Field::Options(_) => {
                            // Could expand inline in the future
                        }
                        _ => {
                            state.editing = true;
                            state.input_buffer = state.get_current_value();
                            if state.input_buffer.starts_with("(none") {
                                state.input_buffer.clear();
                            }
                        }
                    }
                }
                KeyCode::Esc => {
                    if *quit_confirm {
                        // Second escape - quit without saving
                        return Ok(());
                    }
                    
                    if state.dirty {
                        // Confirm quit with unsaved changes
                        *quit_confirm = true;
                        state.message = "Unsaved changes! Press Esc again to quit without saving.".to_string();
                    } else {
                        // No changes - quit
                        return Ok(());
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if key.modifiers.contains(event::KeyModifiers::CONTROL) || !state.dirty {
                        // Ctrl+S or 's' when not dirty - save
                        if let Err(e) = state.save() {
                            state.message = e;
                        }
                        *quit_confirm = false;
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    *quit_confirm = false;
                    if state.selected_field == Field::Options(0) || 
                       matches!(state.selected_field, Field::OptionFlag(_, _) | Field::OptionCommand(_, _)) {
                        // Add new option
                        state.manifest.options.push(OptionSwitch {
                            flags: vec!["new-flag".to_string()],
                            _is_default: false,
                            commands: vec!["new-command".to_string()],
                        });
                        state.dirty = true;
                        state.selected_field = Field::Options(state.manifest.options.len() - 1);
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    *quit_confirm = false;
                    if state.manifest.options.len() > 1 {
                        if let Field::Options(idx) = state.selected_field {
                            state.manifest.options.remove(idx);
                            state.dirty = true;
                            if idx >= state.manifest.options.len() {
                                state.selected_field = Field::Options(state.manifest.options.len().saturating_sub(1));
                            }
                        }
                    } else {
                        state.message = "Cannot delete the last option.".to_string();
                    }
                }
                _ => {
                    *quit_confirm = false;
                }
            }
        }
    }
}

/// Render the form on the left panel
fn render_form(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
    let fields = state.get_fields();
    let current_idx = fields.iter().position(|f| f == &state.selected_field).unwrap_or(0);
    
    let mut lines = vec![
        Line::from(Span::styled("=== Module Editor ===", Style::default().fg(Color::Yellow).bold())),
        Line::from(""),
    ];
    
    for (i, field) in fields.iter().enumerate() {
        let is_selected = i == current_idx;
        let is_editing = is_selected && state.editing;
        
        let (label, value) = match field {
            Field::Name => ("Name", state.manifest.name.clone()),
            Field::Aliases => ("Aliases", state.manifest.aliases.join(", ")),
            Field::Executable => (
                "Executable",
                if state.manifest.executable.is_empty() {
                    "(none - command-only)".to_string()
                } else {
                    state.manifest.executable.clone()
                },
            ),
            Field::Options(idx) => {
                if *idx < state.manifest.options.len() {
                    let opt = &state.manifest.options[*idx];
                    ("Options", format!("[{} flag(s), {} command(s)]", opt.flags.len(), opt.commands.len()))
                } else {
                    ("Options", "(none)".to_string())
                }
            }
            Field::OptionFlag(opt_idx, flag_idx) => {
                ("", format!("  Flag: {}", 
                    if *opt_idx < state.manifest.options.len() && *flag_idx < state.manifest.options[*opt_idx].flags.len() {
                        state.manifest.options[*opt_idx].flags[*flag_idx].clone()
                    } else {
                        "(none)".to_string()
                    }
                ))
            }
            Field::OptionCommand(opt_idx, cmd_idx) => {
                ("", format!("  Command: {}", 
                    if *opt_idx < state.manifest.options.len() && *cmd_idx < state.manifest.options[*opt_idx].commands.len() {
                        state.manifest.options[*opt_idx].commands[*cmd_idx].clone()
                    } else {
                        "(none)".to_string()
                    }
                ))
            }
        };
        
        let display_value = if is_editing {
            format!("{}{}", state.input_buffer, " ".to_string())
        } else {
            value
        };
        
        let field_style = if is_selected {
            if is_editing {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            }
        } else {
            Style::default().fg(Color::White)
        };
        
        if !label.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<12}", format!("{}:", label)), field_style),
                Span::styled(display_value, field_style),
            ]));
        } else {
            lines.push(Line::from(Span::styled(display_value, field_style)));
        }
    }
    
    let content = Paragraph::new(lines)
        .scroll((0, 0));
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Form")
        .style(Style::default().fg(Color::White));
    
    f.render_widget(block, area);
    f.render_widget(content, area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 }));
}

/// Render the XML preview on the right panel
fn render_preview(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
    let preview = state.generate_preview();
    
    let paragraph = Paragraph::new(preview)
        .style(Style::default().fg(Color::LightGreen))
        .scroll((0, 0));
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Live XML Preview")
        .style(Style::default().fg(Color::White));
    
    f.render_widget(block, area);
    f.render_widget(paragraph, area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 }));
}

/// Render the bottom bar with help and status
fn render_bottom_bar(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
    // Help text
    let help = if state.editing {
        "Enter: Apply  Esc: Cancel  |  Tab/Shift+Tab: Navigate"
    } else {
        "Tab/Shift+Tab: Navigate  Enter: Edit  Ctrl+S: Save  Esc: Quit"
    };
    
    let dirty_indicator = if state.dirty { " [MODIFIED]" } else { "" };
    let message = if state.message.is_empty() {
        format!("{}{}", help, dirty_indicator)
    } else {
        format!("{} | {}", state.message, dirty_indicator)
    };
    
    let paragraph = Paragraph::new(message)
        .style(if state.message.contains("error") || state.message.contains("Error") || state.message.contains("fail") {
            Style::default().fg(Color::Red)
        } else if state.message.contains("Saved") || state.message.contains("success") {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        });
    
    f.render_widget(paragraph, area);
}
