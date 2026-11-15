# Your first pack

A walk from nothing to a girl you can see in the game. It takes about half an hour, most of it spent on her art.

## 1. Make the project

On the home page choose **New project**. Give it a name, say what it will hold (tick **girls** for this walk), and make it. You land in the workspace with the folders made and nothing in them.

Keep the namespace as `pleasurehorizons`. A girl's own files are always looked up there.

## 2. Start with the wizard

Press the plus button, choose **Girl**, and use the new girl wizard. It asks:

1. **Who she is**: her id (lower case letters, digits and underscores, like `amber`), her name and what kind of girl she is ([the types](doc:girls)).
2. **Her skeleton**: the body type her animations were made for. This decides which scenes fit her. If you are drawing her from scratch pick the closest of the game's skeletons; if she is a retexture, pick her original's.
3. **Her models**: a nude and a dressed `.strutmodel` (one of them is enough: a girl with only a nude model, like the Kobold, is always drawn nude, and one with only a dressed model always dressed; for a girl who is always nude, also set `wears_clothes` to `false` in her `body` component, so the game never tries to dress or undress her). They are checked against the skeleton's bones, and you can start from a game girl's models as a base.
4. **Write the files**: the wizard writes her girl file, models, texture and a base animation file together.

Open her girl file afterwards. It opens as a **Form**; switch to **JSON** to see the same file as text.

## 3. Look at her

Open her `.strutmodel` from `asset_sync/strut/models/`. The viewer draws her with her texture. If she has animations in the pack, pick a clip, press Play and watch the cues it fires listed under it. Click a cue to jump to its moment.

## 4. Give her a personality and behaviour

In the girl form:

- **Personality** (shy, bold, motherly, bratty or stoic) biases her likes and dislikes. They are rolled once for each girl.
- **Traits** decide how she behaves: how she moves, how she meets strangers, how she fights. The form lists them by category. Leave a category out and she gets its default.

Read [trait files](doc:traits) when you want one of your own.

## 5. Check the problems

Open the Problems panel. A new girl usually shows warnings: a missing texture, a model nothing animates. Fix what it names; many have a quick fix. When it says **No problems found**, the game will accept the pack.

## 6. Give her a scene

Make a **Scene** from the plus button. Choose a type (on a bed, say), the skeleton, and tags. Then press **Simulate** to watch it play: she starts dressed, strips on the right cue, goes to a bed, and the game's own planner picks her clips as you thrust. The cue list shows every sound and chat line that fires.

If the scene has no animation for her skeleton yet, nothing will play: check the file names against [which file a girl plays](doc:animations).

## 7. Try it in the game

Press **Try in a world** and choose the world's folder (the one with `level.dat`). The pack is copied into that world's `datapacks` folder. Open the world, run `/reload`, and spawn her with her spawn egg (or `/pleasure spawn`). Models, textures and animations are sent to players when they join.

If something is wrong, press **Game log**. It lists what the game said about your files, each tied to the file it names.

## 8. Share it

**Export** writes the pack as a zip. Send it to others; they drop it into a world's `datapacks` folder.

## What next

- Fill her out with [dialogue](doc:dialogue) and [cues and sounds](doc:cues).
- Learn how [scene animation files](doc:animations) are picked, so one animation can serve a whole skeleton.
- Open [the tools guide](doc:tools) for the viewer, cue editor, importer and History.
