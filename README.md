# GD Manager (gdm)

**GD Manager** (`gdm`) is a CLI tool for managing Godot plugin dependencies.

## Asset Management and Caching

The `gdm` CLI uses a `.gdm` directory in your project as a cache for downloaded plugin assets. This folder temporarily stores assets before they are extracted and used by your project.

Since `.gdm` is managed automatically (assets are downloaded and removed as needed), you should add `.gdm` to your `.gitignore` file to avoid committing cached or temporary files to version control.

**Example `.gitignore` entry:**
```
.gdm/
```

## Important: Managing Plugins with `gdm`

When using `gdm`, all plugin additions and removals should be performed through the CLI tool. Directly editing the `project.godot` file to add or remove plugins is not supported and may result in inconsistencies or overwritten changes.

`gdm` manages the `[plugin]` section of your `project.godot` file automatically. Any manual changes to plugin entries in this file may be lost the next time you run a `gdm` command that modifies plugins. Always use `gdm add` or `gdm remove` to ensure your project stays in sync and your plugin configuration is preserved correctly.

## Features

- Add plugins to your project
- Install all listed plugins
- Update plugins to their latest versions
- Check for outdated plugins
- Search for plugins
- Remove plugins

## Installation

Download the latest release for your platform from the [GitHub Releases page](https://github.com/k0psutin/gdm/releases).

1. Go to the [releases page](https://github.com/k0psutin/gdm/releases).
2. Download the appropriate binary for your operating system.
3. Extract the downloaded archive.

### Installation Methods

You can install `gdm` either globally (available system-wide) or locally (just for your project).

#### Install Globally

This makes `gdm` available from any terminal window.

##### Linux

```sh
tar -xzf gdm-x86_64-linux-gnu.tar.gz
sudo mv gdm /usr/local/bin/
```

##### Windows

Extract the `.zip` file and move `gdm.exe` to a folder in your `PATH` (e.g., `C:\gdm`), or add the extracted folder to your `PATH`.

##### macOS

```sh
tar -xzf gdm-x86_64-apple-darwin.tar.gz
sudo mv gdm /usr/local/bin/
```

#### Install Locally (Project Folder)

You can place the `gdm` binary directly inside your Godot project directory and run it from there without installing globally. This is useful if you want to keep the tool project-specific or don't have permission to install system-wide.


## Usage

Run `gdm <command> [options]` in your Godot project directory.

### Commands

#### `add`

Add a plugin dependency to your project.

```sh
gdm add <plugin-name>
```

You can optionally specify the `--asset-id` and `--version` flags:

```sh
gdm add <plugin-name> [--asset-id <godot-asset-id>] [--version <version>]
```

- `--asset-id`: Specify the Godot Asset ID for the plugin.
- `--version`: Specify the version of the plugin to add.

> **Note:** If you add a plugin that is already present, `gdm` will attempt to update it to the specified (or latest) version. To install a specific older version, use the `--version` flag with the desired version number.

#### `install`

Install all plugin dependencies listed in your project.

```sh
gdm install
```

#### `update`

Update all plugins to their latest versions.

```sh
gdm update
```

#### `outdated`

List plugins that have newer versions available.

```sh
gdm outdated
```

**Example output:**

```
Checking for outdated plugins

Plugin                Current   Latest
gut                   9.1.0     9.5.0 (update available)
interactive-grid      1.0.0     1.0.0
```

#### `search`

Search for plugins by plugin title.

```sh
gdm search <asset-name>
```

You can optionally specify the `--godot-version` flag to filter results by Godot version:

```sh
gdm search <asset-name> --godot-version <version>
```

- `--godot-version`: Only show plugins compatible with the specified Godot version.

By default, the CLI tries to determine the Godot version used in your project from the `project.godot` file. If it cannot detect the version automatically, or if you want to search or install plugins for a different or older Godot project, you should use the `--godot-version` flag to specify the desired version explicitly.

#### `remove`

Remove a plugin dependency from your project.

```sh
gdm remove <plugin-name>
```

> **Note:** The `<plugin-name>` must match the name of the plugin as it appears in your `gdm.json` file.

## Example Workflow

```sh
# Add a plugin
gdm add "Godot Unit Testing"

# Install all plugins
gdm install

# Check for outdated plugins
gdm outdated

# Update plugins
gdm update

# Search for a plugin
gdm search dialogue

# Remove a plugin
gdm remove godot-dialogue-manager
```

## License

MIT
