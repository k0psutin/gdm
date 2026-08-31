# Update documentation GIFs

## Requirements

- vhs -  [https://github.com/charmbracelet/vhs](https://github.com/charmbracelet/vhs)
- gdm

> NOTE: docs folder already includes `project.godot` and `gdm.toml` to fiddle with.

## Update all .gifs

```bash
./generate_gifs.sh
```

## Updating a single .gif

To update a specific `.gif` go to `docs` and run related `.tape`:
```bash
cd docs
vhs vhs/gdm_add.tape
```

> NOTE: Some gifs might need some fiddling with `gdm.toml`, like `gdm_update.tape` and `gdm_outdated.tape`.

You can search older versions of a dependency by selecting it in the `Godot Asset Library` and clicking `Recent edits`.
