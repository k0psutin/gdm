![Version badge](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fk0psutin%2Fgdm%2Frefs%2Fheads%2Fmain%2FCargo.toml&query=%24.package.version&label=version)
![Coverage](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgist.githubusercontent.com%2Fk0psutin%2F02a7627bd5ba7bdaaf0063e02cadcfde%2Fraw%2F7cf2de6525c551d8d68af57847fbb9713323a6a3%2Fgdm_coverage.json&query=%24.coverage&suffix=%25&label=coverage&color=green)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.94.0%2B-orange.svg)

# GD Manager (gdm)

**GD Manager** (`gdm`) is a CLI tool for managing Godot dependencies, similar to npm or cargo for Godot projects.

## Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
- [Supported Godot Versions](#supported-godot-versions)
- [Installation](#installation)
- [Usage](#usage)
  - [add](#add)
  - [install](#install)
  - [list](#list)
  - [update](#update)
  - [outdated](#outdated)
  - [search](#search)
  - [remove](#remove)
- [Examples](#examples)
- [Bug Reports and Issues](#bug-reports-and-issues)
- [License](#license)

## Quick Start

1. Download `gdm` for your platform from the [releases page](https://github.com/k0psutin/gdm/releases)
2. Place the binary in your PATH or project directory
3. Navigate to your Godot project directory
4. Search for a dependency: `gdm search "dialogue"`
5. Add the dependency: `gdm add "Dialogue Manager"`
6. Install: `gdm install`

That's it! Your dependency is now installed and ready to use.

## Features

- Search for dependencies from the Godot Asset Library
- Add dependencies from the Godot Asset Library or Git repositories
- Install all project dependencies with one command
- List project dependencies
- Update dependencies to their latest versions
- Check for outdated dependencies
- Remove dependencies cleanly
- Automatic `project.godot` management
- Dependency tracking via `gdm.toml`
- Support for Git-based dependencies

## Supported Godot Versions

- 3.6.x
- 4.5.x
- 4.6.x

## Temporary Directory

`gdm` creates a `.gdm` directory to temporarily store downloaded dependency archives. Add this to your `.gitignore`:

**Example `.gitignore` entry:**
```bash
.gdm
```

## Important: Managing Dependencies with `gdm`

When using `gdm`, **all dependency additions and removals should be performed through the CLI**. Manual editing of `project.godot` is not supported and may cause inconsistencies.

### How It Works

- `gdm` automatically manages the `[editor_plugins]` section in `project.godot`
- Dependency metadata is stored in `gdm.toml` under the `[dependencies]` table
- Manual changes to dependency entries may be overwritten by `gdm` commands

For example:

```toml
[dependencies.netfox]
title = "netfox"
version = "v1.35.3"

[dependencies.netfox.source]
publisher_slug = "foxssake"
asset_slug = "netfox"
```

### Migration from Manual Dependency Management

> **Important:** There is no automatic migration path for existing dependencies. To use `gdm` with a project that already has dependencies:
> 
> 1. Note your current dependencies
> 2. Remove them manually from `project.godot` and `/addons`
> 3. Reinstall via `gdm add` and `gdm install`
> 
> This ensures `gdm.toml` and `project.godot` stay synchronized.

### Dependencies with Multiple Folders

If a downloaded dependency contains multiple folders in `/addons`, `gdm` automatically identifies the main dependency for `gdm.toml`. Additional folders are marked as `sub_assets`.

Projects using the legacy `gdm.json` manifest require a manual conversion to `gdm.toml`. Create the `[dependencies]` entries shown above and copy the dependency fields into the new file. `gdm` ignores the legacy file and never migrates or deletes it.

## Installation

Download the latest release for your platform from the [GitHub Releases page](https://github.com/k0psutin/gdm/releases).

### Installation Methods

Choose between global installation (accessible from anywhere) or local installation (project-specific).

#### Global Installation (Recommended)

Makes `gdm` available system-wide from any terminal.

**Linux:**

```bash
tar -xzf gdm-linux-x86_64.tar.gz
sudo mv gdm /usr/local/bin/
```

**macOS:**

```bash
tar -xzf gdm-macos-aarch64.tar.gz
sudo mv gdm /usr/local/bin/
```

**Windows:**

1. Extract `gdm-windows.zip`
2. Move `gdm.exe` to a folder in your `PATH` (e.g., `C:\Program Files\gdm`)
3. Or add the extracted folder to your system `PATH`

#### Local Installation (Project-Specific)

Place the `gdm` binary in your Godot project directory. Useful for:
- Project-specific tooling without system-wide installation
- Environments where you don't have admin/sudo privileges
- Keeping different `gdm` versions per project

## Usage

Run `gdm <command> [options]` in your Godot project directory.

![gdm intro](./docs/gifs/gdm_intro.gif)

### Commands

#### `add`

Add a dependency to your project from the Godot Asset Library.

**Note:** This command will also install the dependency.

**Basic usage:**

```bash
gdm add '<dependency-name>'
```

![gdm add](./docs/gifs/gdm_add.gif)

**With optional flags:**

```bash
gdm add '<dependency-name>' [--asset-id <godot-asset-id>] [--version <version>]
```

**Flags:**
- `--asset-id`: Specify the Godot Asset Library ID (useful when dependency name is ambiguous)
- `--version`: Install a specific version instead of the latest

**Adding from Git repositories:**

```bash
gdm add --git <git-url> --ref <branch-or-tag-or-commit>
```

**Flags:**
- `--git`: Git repository URL (HTTPS or SSH)
- `--ref`: Branch name (e.g., `main`), tag (e.g., `v1.2.3`), or commit hash (e.g., `abc123`)

![gdm add git](./docs/gifs/gdm_add_git.gif)

**Examples:**
```bash
# Add from Asset Library
gdm add "Dialogue Manager"
gdm add "Dialogue Manager" --version "3.1.0"

# Add from Git using branch
gdm add --git https://github.com/username/godot-dependency.git --ref main

# Add from Git using tag  
gdm add --git https://github.com/username/godot-dependency.git --ref v1.2.3

# Add from Git using commit hash
gdm add --git https://github.com/username/godot-dependency.git --ref a1b2c3d
```

> **Note:** When adding a dependency that already exists, `gdm` will update it to the specified version. Git dependencies are **not** auto-updated by `gdm update` - you must manually remove and re-add them with a new `--ref` to update.

#### `install`

Install all dependencies listed in `gdm.toml`.

```bash
gdm install
```

![gdm install](./docs/gifs/gdm_install.gif)

#### `list`

List the dependencies declared in `gdm.toml`.

```bash
gdm list
```

The command lists one row for each top-level dependency, sorted by its manifest key. Asset Store versions and Git references are shown in the `Version` column. Git sources are displayed without a leading `http://` or `https://`, one trailing `/`, or one trailing `.git`.

If no `gdm.toml` exists or it contains no dependencies, the command prints `No dependencies found.`. A legacy `gdm.json` is ignored. The command does not require `project.godot`.

```text
Dependency          Key              Version        Source
netfox              netfox           v1.35.3        foxssake/netfox

To remove a dependency, use: gdm remove <key>
```

#### `update`

Update all Godot Asset Library dependencies to their latest versions.

```bash
gdm update
```

![gdm update](./docs/gifs/gdm_update.gif)

> **Note:** Dependencies installed via Git (`--git` flag) will not be updated by this command.

#### `outdated`

Check which Godot Asset Library dependencies have newer versions available.

```bash
gdm outdated
```

![gdm outdated](./docs/gifs/gdm_outdated.gif)

> **Note:** Dependencies installed via Git (`--git` flag) will not be shown by this command.

#### `search`

Search the Godot Asset Library for dependencies.

```bash
gdm search '<dependency-name>'
```

![gdm search](./docs/gifs/gdm_search.gif)

**With Godot version filter:**

```bash
gdm search '<dependency-name>' --godot-version '<version>'
```

**Example:**
```bash
gdm search "dialogue" --godot-version "4.3"
```

#### `remove`

Remove a dependency from your project.

```bash
gdm remove '<dependency-key>'
```

![gdm remove](./docs/gifs/gdm_remove.gif)

> **Note:** The `<dependency-key>` must match the dependency key as it appears in your `gdm.toml` file.

## Examples

### Setting Up a New Project

```bash
# Initialize your Godot project first in Godot Editor
cd my-godot-project

# Search for dependencies
gdm search "dialogue manager"

# Add dependencies
gdm add "Dialogue Manager 3"
gdm add "Godot Unit Testing"
```

### Cloning an Existing Project

```bash
# Clone the project
git clone https://github.com/username/godot-game.git
cd godot-game

# Install all dependencies
gdm install
```

### Updating Dependencies

```bash
# Check for updates
gdm outdated

# Update all dependencies
gdm update
```

## Bug Reports and Issues

Found a bug or have a feature request? Please [create an issue](https://github.com/k0psutin/gdm/issues) on GitHub.

**When reporting bugs, please include:**
- Your operating system (Linux, macOS, Windows)
- Your Godot version
- `gdm` version (shown with `gdm --version`)
- Steps to reproduce the issue
- Error messages or logs
- Your `gdm.toml` file (if relevant)

**For feature requests:**
- Describe the feature and why it would be useful
- Provide examples of how it would work

## License

MIT
