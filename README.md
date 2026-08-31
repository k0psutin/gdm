![Version badge](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fk0psutin%2Fgdm%2Frefs%2Fheads%2Fmain%2FCargo.toml&query=%24.package.version&label=version)
![Coverage](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgist.githubusercontent.com%2Fk0psutin%2F02a7627bd5ba7bdaaf0063e02cadcfde%2Fraw%2F7cf2de6525c551d8d68af57847fbb9713323a6a3%2Fgdm_coverage.json&query=%24.coverage&suffix=%25&label=coverage&color=green)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.96.0%2B-orange.svg)

# GD Manager (gdm)

**GD Manager** (`gdm`) is a CLI tool for managing Godot dependencies, similar to npm or cargo for Godot projects.

## Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
- [Supported Godot Versions](#supported-godot-versions)
- [Dependency Manifest](#dependency-manifest)
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
- [Development](#development)
- [Bug Reports and Issues](#bug-reports-and-issues)
- [License](#license)

## Quick Start

1. Download `gdm` for your platform from the [releases page](https://github.com/k0psutin/gdm/releases)
2. Place the binary in your PATH or project directory
3. Navigate to a Godot project directory containing `project.godot`
4. Search for a dependency: `gdm search "dialogue"`
5. Add the dependency: `gdm add "Dialogue Manager"`

`gdm add` records and installs the dependency immediately. Use `gdm install` later to install all dependencies recorded in an existing `gdm.toml`.

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

`gdm` reads the Godot version from `project.godot` when querying the Asset Library. The project has been tested with:

- Godot 3.6.x (`config_version=4`)
- Godot 4.5.x and 4.6.x (`config_version=5`)

Use `--godot-version` with `search` or `add` to override the detected version when necessary.

## Temporary Directory

`gdm` creates a `.gdm` directory to temporarily store downloaded dependency archives. Add this to your `.gitignore`:

**Example `.gitignore` entry:**
```bash
.gdm
```

## Dependency Manifest

`gdm.toml` is the canonical dependency manifest. When using `gdm`, **all dependency additions and removals should be performed through the CLI**. Manual editing of `project.godot` is not supported and may cause inconsistencies.

### How It Works

- `gdm` automatically manages the `[editor_plugins]` section in `project.godot`
- Dependency metadata is stored in `gdm.toml` under the `[dependencies]` table
- The manifest key is used by `gdm list` and `gdm remove`; it normally matches the dependency folder name
- `gdm` may rewrite dependency entries when installing or updating them

For example:

```toml
[dependencies.netfox]
title = "netfox"
version = "v1.35.3"

[dependencies.netfox.source]
publisher_slug = "foxssake"
asset_slug = "netfox"
```

Asset Library dependencies can also be selected directly without a name search:

```bash
gdm add foxssake/netfox
gdm add --publisher-slug foxssake --asset-slug netfox
```

Git dependencies store the repository URL and the selected branch, tag, or commit:

```toml
[dependencies.my_dependency]
plugin_cfg_path = "addons/my_dependency/plugin.cfg"
title = "My dependency"
version = ""
sub_assets = []

[dependencies.my_dependency.source]
url = "https://github.com/example/my-dependency.git"
reference = "main"
```

The remaining fields are managed by `gdm`: `plugin_cfg_path` identifies the installed plugin configuration, `sub_assets` records additional folders found under `addons`, and `license` stores the Asset Library license when available.

### Migrating an Existing Project

`gdm` does not import arbitrary existing addons. Recreate each dependency through the CLI so the new Asset Library API resolves its current metadata and installation layout:

1. Search for each Asset Library dependency with `gdm search '<dependency-name>'`.
2. Add it with `gdm add '<publisher>/<asset>'` or the `--publisher-slug` and `--asset-slug` options.
3. Add Git dependencies with `gdm add --git <git-url> [--ref <reference>]`.

Each `gdm add` installs the dependency immediately, creates or updates `gdm.toml`, and keeps the project plugin configuration synchronized. Use `gdm list` to review the recreated manifest; `gdm install` is only needed later when installing dependencies from an existing manifest, such as after cloning a project.

Do not manually edit `project.godot` or convert the old manifest by hand.

### Dependencies with Multiple Folders

If a downloaded dependency contains multiple folders in `/addons`, `gdm` automatically identifies the main dependency for `gdm.toml`. Additional folders are marked as `sub_assets`.

Projects using the legacy `gdm.json` manifest should recreate their dependencies with `gdm add`; do not copy its fields into `gdm.toml`. `gdm` ignores the legacy file and never migrates or deletes it.

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
gdm add '<dependency-name>' [--version <version>] [--godot-version <version>]
```

**Flags:**
- `--version`: Install a specific Asset Library version instead of the latest
- `--godot-version`: Override the Godot version detected from `project.godot`
- `--publisher-slug` and `--asset-slug`: Select an Asset Library dependency by its exact publisher and asset slugs

The exact publisher/asset identity can be supplied either as `publisher/asset` or with both slug flags:

```bash
gdm add 'foxssake/netfox'
gdm add --publisher-slug foxssake --asset-slug netfox
```

**Adding from Git repositories:**

```bash
gdm add --git <git-url> [--ref <branch-or-tag-or-commit>]
```

**Flags:**
- `--git`: Git repository URL (HTTPS or SSH)
- `--ref`: Branch name (e.g., `main`), tag (e.g., `v1.2.3`), or commit hash (e.g., `abc123`). Defaults to `main`.

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

> **Note:** When adding a dependency that already exists, `gdm` will update it to the specified version. Git dependencies are **not** checked by `gdm update` or `gdm outdated`; remove and re-add them with a new `--ref` to change the selected revision.

#### `install`

Install all dependencies listed in `gdm.toml`.

This command requires both `project.godot` and at least one dependency in `gdm.toml`.

```bash
gdm install
```

![gdm install](./docs/gifs/gdm_install.gif)

#### `list`

List the dependencies declared in `gdm.toml`.

```bash
gdm list
```

![gdm list](./docs/gifs/gdm_list.gif)

The command lists one row for each top-level dependency, sorted by its manifest key. The manifest key is shown in the `Dependency` column. Asset Store versions and Git references are shown in the `Version` column. Git sources are displayed without a leading `http://` or `https://`, one trailing `/`, or one trailing `.git`.

If no `gdm.toml` exists or it contains no dependencies, the command prints `No dependencies found.`. A legacy `gdm.json` is ignored. The command does not require `project.godot`.

```text
Dependency  Version  Source
netfox      v1.35.3  foxssake/netfox

To remove a dependency, use: gdm remove <dependency>
```

#### `update`

Update all Godot Asset Library dependencies to their latest versions.

Git dependencies are left unchanged. If all dependencies are already current, the command reports that no update is needed.

```bash
gdm update
```

![gdm update](./docs/gifs/gdm_update.gif)

> **Note:** Dependencies installed via Git (`--git` flag) will not be updated by this command.

#### `outdated`

Check which Godot Asset Library dependencies have newer versions available.

Git dependencies are not included in this check. Use `gdm update` to apply available Asset Library updates.

```bash
gdm outdated
```

![gdm outdated](./docs/gifs/gdm_outdated.gif)

> **Note:** Dependencies installed via Git (`--git` flag) will not be shown by this command.

#### `search`

Search the Godot Asset Library for dependencies. Results show the publisher/asset identifier used by `gdm add`, the available version, license, net review score, and installation status.

```bash
gdm search '<dependency-name>'
```

![gdm search](./docs/gifs/gdm_search.gif)

**With Godot version filter:**

```bash
gdm search '<dependency-name>' --godot-version '<version>'
```

**With an Asset Library version filter:**

```bash
gdm search '<dependency-name>' --version '<version>'
```

**Example:**
```bash
gdm search "dialogue" --godot-version "4.3"
```

```text
#  Dependency                                      Version      License      Score  Status
1  mikeschulze/gdunit4-unit-testing-framework      v6.2.0       MIT          +2     installed
   An testing framework designed for testing GdScripts, C# scripts, and scenes...
```

#### `remove`

Remove a dependency from your project.

```bash
gdm remove '<dependency-key>'
```

![gdm remove](./docs/gifs/gdm_remove.gif)

> **Note:** The `<dependency-key>` must match the dependency key as it appears in your `gdm.toml` file. `gdm remove` also removes the dependency from `project.godot` and deletes its installed addon folders when they are no longer shared by another dependency.

## Examples

### Setting Up a New Project

```bash
# Initialize your Godot project first in Godot Editor
cd my-godot-project

# Search for dependencies
gdm search "dialogue manager"

# Add dependencies
gdm add "Dialogue Manager 3"
gdm add "GDUnit4"
# Each add also installs the dependency and updates gdm.toml
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

### Inspecting Dependencies

```bash
# List the manifest keys, versions, and sources
gdm list

# Remove a dependency by its manifest key
gdm remove <dependency-key>
```

## Development

- [Run tests and coverage](./tests/README.md)
- [Update the documentation GIFs](./docs/README.md)

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
