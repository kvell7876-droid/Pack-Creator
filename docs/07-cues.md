# Cues and sounds

A **cue** is a name a marker in an animation fires at a moment: `moan`, `thrust`, `becomenude`. A marker is one or more cue names joined by dashes, so `moan-thrust` fires both. The cues the game's own animations fire are listed in [Cues and sounds](ref:cues). A binding for a cue that is in no animation never plays, and the viewer flags it.

Cues are how an animation talks to the rest of the game: the animation says *when*, and the girl's bindings say *what happens*.

## Event bindings

`asset_sync/strut/event_bindings/girls/<girl>.json` says what her cues do:

```json
{
  "bindings": {
    "moan": {
      "sounds": ["pleasurehorizons:amber.moan"],
      "random_sounds": [],
      "pleasurehorizons:messages": ["Hihi~"]
    }
  }
}
```

| Key | Does |
|---|---|
| `sounds` | All of these play together |
| `random_sounds` | One of these plays, chosen at random |
| `pleasurehorizons:messages` | Chat lines the girl says, written out in full |
| `pleasurehorizons:player_messages` | The same for lines said as the player |

Because every girl has her own bindings file, two girls can react to the same cue in their own way: the same `moan` marker plays Lucy's voice for Lucy and yours for your girl.

## Adding a line

1. Open the animation in the viewer and find the moment you want. Add a cue there, or reuse one that is already in the clip.
2. Open her bindings file and add the cue's name with `messages`.
3. Run the problems check. A cue the animation does not fire, or a sound that does not exist, is flagged and the nearest real name is offered.

You never write the chat line in the animation; it lives in the bindings so it can be edited, translated and replaced without touching a binary file.

## Old keyframe event files

Older packs kept chat lines in `keyframe_events/<girl>.json`. **The game no longer reads that file.** The checks warn about one, and **Import** puts its lines into the bindings. Delete the old file once the lines are moved.

## Sounds

Sound files are `.ogg`, declared in `asset_sync/sounds.json`. The sound events the game already has are listed in the reference, and a name that is close to one is offered as a repair. A sound name in a binding is a *sound event*: the id declared in `sounds.json`, not the file's path.

Sounds are sent to every player who joins, so keep them short: there is a size limit ([assets](doc:assets)).

## Language files

Names and other text the game looks up by key go in a language file under `asset_sync/lang/` (`en_us.json`). A scene's `display_name` is such a key.

## Common problems

- **A cue that never plays**: it is in the bindings but not in any animation, or the spelling differs by a letter.
- **A sound that is silent**: it is not declared in `sounds.json`, or the `.ogg` is missing.
- **A line that shows twice**: two cues in one marker both carry the same message.
