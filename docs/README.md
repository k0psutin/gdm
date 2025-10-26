# Update documentation GIFs

## Requirements

- vhs -  [https://github.com/charmbracelet/vhs](https://github.com/charmbracelet/vhs)
- gdm

> NOTE: docs folder already includes `project.godot` and `gdm.json` to fiddle with.

## Updating gifs

To update a specific `.gif` go to `docs` and run related `.tape`:
```bash
cd docs
vhs vhs/gdm_add.tape
```

> NOTE: Some gifs might need some fiddling with `gdm.json`, like `gdm_update.tape` and `gdm_outdated.tape`. 

You can search older version of an asset by selecting the asset at the `Godot Asset Library` and clicking `Recent edits`.