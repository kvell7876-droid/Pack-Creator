# Scene animations

Animations are `.strutanim` files in `data/pleasurehorizons/asset_sync/strut/animations/`. They are binary: make them in Blockbench with the Strut Toolkit (the **Blockbench** button installs both) and save them to the pack. The Pack Creator never asks you to write one by hand.

## Which file a girl plays

A scene's clips are looked up in the first file that exists, from most specific to least. These live in the `scenes/` folder:

| File | Played by |
|---|---|
| `<scene>-<girl>.strutanim` | that one girl (`cowgirl-momo`) |
| `<scene>.<skeleton>.strutanim` | every girl on that skeleton (`cowgirl.bia`) |
| `<scene>.strutanim` | everyone |

**Her own file wins over her skeleton's, and her skeleton's wins over the shared one.** So `cowgirl-momo` is played by Momo even though she is a `bia` model, the other `bia` girls play `cowgirl.bia`, and everybody else plays `cowgirl`.

The skeleton is written with a **dot**, not braces. A resource name may only contain `a-z 0-9 _ - . /`, so `cowgirl{bia}.strutanim` cannot be loaded by the game; the checks tell you the name that works.

Her everyday animations are in `base.strutanim` (shared) and `base.<girl>.strutanim` (hers).

### Choosing where to put an animation

- Animate for a **skeleton** when the same motion is right for every girl built on it. One file then serves them all, and a new girl on that skeleton gets the scene for free.
- Animate for **one girl** only when she differs: a different height, a prop, a unique pose.
- Make the **shared** file only when the animation works on every skeleton, which is rare.

`replace_default_animations` in the scene file controls whether a girl's own file replaces the shared one entirely or is used alongside it ([scene files](doc:scenes)).

## Clip names

A clip in a scene file is named `animation.<scene>.<role>`. A scene with several characters has a clip for each role (the girl, the player). The slow and fast loops the pace game switches between are separate clips. A clip that does not follow the pattern is an error.

## Markers (cues)

A clip can carry markers at points in time. A marker is a dash-separated list of cue names (`moan-thrust`). What each cue does is set by bindings: sounds to play, lines to say. See [cues and sounds](doc:cues).

A few cues are special to the game itself, such as `becomenude` (she undresses) and `thrust` (the beat the pace game rewards). The reference lists every cue the game's own animations use.

## Looking at one

Opening a `.strutanim` in the viewer plays any clip, lists its cues and lets you add, move, retime and remove them ([the cue editor](doc:tools)). **Edit in Blockbench** opens it on its model.

## Common problems

- **A scene plays nothing for a girl**: check the file name against the table above, and that the skeleton is spelled as in her girl file.
- **Clips not found**: the clip names must start `animation.<scene>.`, with the scene id from the file name.
- **A girl does nothing in a scene**: her bones differ from the skeleton the animation was made for.
