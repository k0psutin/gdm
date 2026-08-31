# Documentation assets

This directory contains the fixture Godot project, VHS tapes, and generated GIFs used by the project README.

## Requirements

- [`vhs`](https://github.com/charmbracelet/vhs)
- A built or installed `gdm` executable available on `PATH`
- A terminal environment supported by VHS

Run the commands below from this directory. The tape files use paths relative to `docs`.

## Fixture project

The tapes operate on `project.godot` and create or update `gdm.toml` while they run. The batch script also performs cleanup commands at the end, so run it from a disposable copy or review the fixture files with `git status` afterward.

The fixture should contain a valid Godot project. `gdm add`, `install`, `update`, `outdated`, and `remove` require `project.godot`; `gdm list` only needs the manifest.

## Generate all GIFs

```bash
cd docs
./generate_gifs.sh
```

Each tape writes to the matching file under `docs/gifs`:

| Tape | Command shown |
| --- | --- |
| `gdm_intro.tape` | `gdm` |
| `gdm_search.tape` | `gdm search` |
| `gdm_add.tape` | `gdm add` |
| `gdm_add_git.tape` | `gdm add --git` |
| `gdm_list.tape` | `gdm list` |
| `gdm_install.tape` | `gdm install` |
| `gdm_remove.tape` | `gdm remove` |
| `gdm_outdated.tape` | `gdm outdated` |
| `gdm_update.tape` | `gdm update` |

## Generate one GIF

```bash
cd docs
vhs vhs/gdm_list.tape
```

Replace `gdm_list.tape` with the tape you want to refresh. Keep the command in the tape, the README example, and the resulting output consistent.

Some recordings depend on the manifest state. In particular, the update and outdated recordings need dependencies with versions that produce the intended output. Adjust only the fixture data needed for the recording and review those changes before committing.

## Review checklist

After recording:

1. Confirm every `Output` path has a corresponding file in `gifs`.
2. Inspect the GIFs for readable output and commands that match the README.
3. Check that fixture changes are intentional with `git status` and `git diff`.
4. Run `git diff --check` before committing.
