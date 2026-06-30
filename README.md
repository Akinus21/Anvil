# Anvil

**Modular CLI Tool Runner** — Turn any script into a polished CLI command.

Anvil lets you package scripts as modules with custom aliases, multiple entry points, and centralized management. No more hunting for that one-off script buried in your dotfiles.

## Features

- **Module-based architecture** — Organize scripts as packages with metadata
- **Multiple commands per module** — Define different flags/entry points
- **Custom aliases** — Short names for your modules
- **Auto-fix doctor** — Diagnoses and repairs configuration issues automatically
- **Interactive module creation** — Build command modules with an interactive prompt
- **Homebrew install** — Easy installation via Homebrew
- **Shell completion** — Tab completion for bash, zsh, and fish
- **Community modules** — Install modules from GitHub repos

## Installation

```bash
brew tap Akinus21/homebrew-tap
brew install anvil
```

After installation, add to your shell config (`~/.bashrc` or `~/.zshrc`):

```bash
export AKTOOLS_HOME="$HOME/.anvil"
export PATH="$AKTOOLS_HOME/bin:$PATH"
```

Then run `anvil doctor` to set everything up.

## Quick Start

### Create a command module interactively

```bash
anvil build-command
# Follow the prompts to create a module with custom flags and commands
```

### Add a script as a module

```bash
anvil add myscript.sh
# Follow the prompts for name and aliases
```

### Run a module

```bash
anvil <module-name> [args...]
```

### List installed modules

```bash
anvil list
```

### Diagnose issues

```bash
anvil doctor        # Auto-fix issues
anvil doctor --no-fix  # Show issues without fixing
```

## Module Structure

Modules live in `~/.anvil/modules/`. Each module is a folder containing:

```
~/.anvil/modules/
└── mymodule/
    ├── manifest.xml
    └── script.sh
```

### manifest.xml

```xml
<?xml version="1.0"?>
<module>
    <name>mymodule</name>
    <alias>mm</alias>
    <executable>./script.sh</executable>
    <option>
        <flag>run</flag>
        <command>./script.sh</command>
    </option>
</module>
```

- `name` — Module identifier
- `alias` — Short command to invoke the module
- `executable` — Path to script (empty for command-only modules)
- `flag` — Command-line flag to match
- `command` — Command(s) to execute

### Command-Only Modules

Modules can be command-only without an executable:

```xml
<?xml version="1.0"?>
<module>
    <name>sys</name>
    <alias>sys</alias>
    <executable></executable>
    <option>
        <flag>upgrade</flag>
        <command>sudo bootc upgrade && reboot</command>
    </option>
</module>
```

Run with `anvil sys upgrade`.

## Commands

| Command | Description |
|---------|-------------|
| `anvil build-command` | Create a new command module interactively |
| `anvil add <file>` | Add a script as a new module |
| `anvil edit [name]` | Edit a module's manifest |
| `anvil edit-aliases` | Edit shell aliases interactively |
| `anvil list` | List all installed modules |
| `anvil rm <name>` | Remove a module |
| `anvil update` | Rebuild the module registry |
| `anvil doctor` | Diagnose and fix configuration issues |
| `anvil completion <shell>` | Generate shell completions (bash/zsh/fish) |
| `anvil add-repo <user/repo>` | Add a GitHub repo to track modules from |
| `anvil list-repos` | List configured repos |
| `anvil search-mods <term>` | Search for modules in repos |
| `anvil install-mods <mod> [<mod>...]` | Install one or more modules from repos |
| `anvil add-mod <module>` | Submit a module to the community repo |
| `anvil autoupdate <sub>` | Manage automatic updates (status/enable/disable/set) |
| `anvil help` | Show this help message |

## Configuration

- **Config directory**: `~/.anvil/`
- **Modules directory**: `~/.anvil/modules/`
- **Registry file**: `~/.anvil/registry.json`
- **Aliases file**: `~/.anvil/aliases.sh`

## Updating

```bash
brew upgrade anvil
```

Or enable automatic updates:

```bash
anvil autoupdate status   # Check current status
anvil autoupdate enable   # Enable daily updates
anvil autoupdate set 12h # Update every 12 hours
```

Supports systemd, launchd (macOS), and cron.

## Shell Completions

Enable tab completion for your shell:

```bash
# Bash
anvil completion bash --install

# Zsh
anvil completion zsh --install

# Fish
anvil completion fish --install
```

## Community Modules

Install modules from GitHub repos:

```bash
# Add a repo to track
anvil add-repo username/my-plugins

# List configured repos
anvil list-repos

# Search for modules
anvil search-mods mymodule

# Install one or more modules (space-separated)
anvil install-mods mymodule anothermod yetanothermod

# Submit your module to the community repo
anvil add-mod mymodule
```

This will:
1. Fork the community repo to your GitHub account
2. Copy your module files into the fork
3. Update registry.json in the fork
4. Create a pull request

When you merge the PR, the module will be available to all anvil users via `anvil install-mods mymodule`.

The default community repo is `Akinus21/anvil-modules` which is always available.

## License

MIT