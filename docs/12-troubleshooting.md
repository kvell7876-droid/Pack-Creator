# Troubleshooting

## The pack does not appear in the game

- Is it in the world's `datapacks` folder? **Try in a world** puts it there. Run `/datapack list` to see it.
- Does the Problems panel show errors? An error stops that file loading. Fix the errors, save, and run `/reload`.
- Is there a `pack.mcmeta` at the pack's root? A new project makes one; a hand-made folder needs one.

## The girl does not appear

- A wild girl with `weight` 0 never spawns by herself. Use her spawn egg, or raise the weight.
- Her `biomes` and `time` may not match where you are.
- Open **Game log**. A girl that failed to load is named there with the reason.

## She looks wrong

- A placeholder model: her id or model path does not match ([assets](doc:assets)).
- Black or wrong texture: the PNG's file name must be her id, in `textures/entities/`.
- A turned arm: use **Fix arms** in the viewer.
- Her item sits wrong in her hand: use `held_item_offset`, `held_item_rotation` and `held_item_scale`.

## A scene does nothing

- Does the scene's `body_types` include her skeleton, and does `supported_girls` or `unsupported_girls` exclude her?
- Is there an animation file for her ([which file she plays](doc:animations))? Check the name, with the skeleton after a **dot**.
- Does she have the affection the scene `requires_affection`?
- Open it in the **simulator**. If it plays there but not in the game, read the Game log.

## Sounds or chat lines do not play

- The cue must be in the animation and in her bindings, spelled the same ([cues](doc:cues)).
- The sound must be declared in `sounds.json` and the `.ogg` must exist.
- A leftover `keyframe_events` file is no longer read. Import it into the bindings.

## Players do not see my models

Assets are sent when a player joins. Ask them to rejoin after you change a model. `/reload` re-reads data files only.

## I broke something

- **History** on the tab restores an earlier version.
- Ctrl+Z undoes the last edit. In the tree, Ctrl+Z undoes a move or a rename.
- A deleted file is in the Recycle Bin.

## Still stuck

Open **Game log**, read the line that names your file, and check the matching guide for that kind of file. The reference pages under Docs list every setting the game accepts.
