# Anvil Rust Refactor - Agent Instructions

## Overview

This is the Rust rewrite of Anvil - a modular CLI tool runner.

## Building

**IMPORTANT: Do NOT install Rust locally.** GitHub Actions handles all building automatically. The CI workflow builds the binary and reports any errors back to you.

If you need to verify code changes without building:
1. Review the code logic manually
2. Check for syntax errors by reading the files
3. Push to GitHub and wait for the build results

## Git Push Workflow

Since gh CLI is not authenticated, use SSH directly:

```bash
cd /home/opencode/projects/anvil
git add -A
git commit -m "<description>"
GIT_SSH_COMMAND="ssh -i /config/.ssh/github -o StrictHostKeyChecking=no" git push origin main
```

**IMPORTANT: Always push to GitHub after making and verifying changes.**

## Documentation Updates

**IMPORTANT: Update README.md when adding new features or changing existing features.**

The README should reflect:
- New commands added
- Changed command behavior
- Updated installation instructions
- New use cases or examples

## Project Structure

```
anvil/
├── Cargo.toml
├── README.md
├── AGENTS.md
└── src/
    ├── main.rs          # Entry point
    ├── registry.rs      # Module registry (JSON)
    ├── modules/
    │   └── mod.rs       # XML manifest parser
    └── commands/
        ├── mod.rs       # Command exports
        ├── add.rs       # Add module from file
        ├── edit.rs      # Edit module manifest
        ├── build_command.rs  # Interactive command module creation
        ├── list.rs      # List modules
        ├── rm.rs        # Remove module
        ├── run.rs       # Run a module
        ├── update.rs    # Rebuild registry
        ├── doctor.rs    # Diagnose issues
        └── help_cmd.rs  # Help text
```

## Module Structure

Modules are stored in `~/.anvil/modules/`:
- Each module is a folder
- Contains `manifest.xml` with metadata
- May contain scripts or resources

### manifest.xml Format

```xml
<?xml version="1.0"?>
<module>
    <name>modulename</name>
    <alias>alias</alias>
    <executable>./script.sh</executable>
    <option>
        <flag>flagname</flag>
        <command>command to run</command>
    </option>
</module>
```

- `<executable>`: Path to script (empty for command-only modules)
- `<flag>`: Command-line flag to match
- `<command>`: Command(s) to execute when flag is used