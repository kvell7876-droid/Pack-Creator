# Trait files

`data/<namespace>/girl_traits/<id>.json`. A trait is a behaviour a girl can be given, such as `archer` or `skittish`. The game ships thirty, and your pack can add its own.

```json
{
  "category": "movement",
  "description": "What she does, in a sentence.",
  "params": {},
  "goals": []
}
```

## Using traits on a girl

A girl uses a trait in her `pleasurehorizons:ai` component, one entry per category. A trait is a name, an object with a `type` and settings, or (where a category takes several) a list:

```json
"pleasurehorizons:ai": {
  "traits": {
    "movement": "pleasurehorizons:walker",
    "combat": { "type": "pleasurehorizons:archer", "max_range": 20 }
  }
}
```

The girl form lists the traits by category with their settings, so you rarely write this by hand. A category you leave out gives her its default.

## The categories

| Category | How many | Default | Decides |
|---|---|---|---|
| `movement` | one | walker | On foot, flying or swimming |
| `temperament` | one | defensive | How she meets strangers and trouble |
| `combat` | one | brawler | How she fights |
| `loadout` | one | unequipped | What she spawns with, from a loot table |
| `interaction` | several | tameable, affectionate, collar | What a player can do with her |
| `routine` | one | sleeper | When she sleeps and what she does idle |
| `territory` | one | roamer | Where she stays |
| `work` | one | worker | Whether a settler does her settlement's work |

Every trait the game ships, with its settings, is on the [Traits](ref:traits) page.

## Writing your own

- `category` is one of the categories above.
- `description` is what the form shows beside it.
- `params` are settings the trait takes. Each has a `type`, a `default`, optional `min` and `max`, and a `description` the form shows. Goals refer to them as `$name`, so the person using the trait can change them in the girl form.
- `goals` are the things she does, each a goal id with its settings and optional conditions. Every goal the game offers is in [Goals](ref:goals).
- `capabilities` switch abilities on or off: `movement`, `fall_damage`, `accepts_collar`, `can_be_tamed`, `takes_gifts` and `loadout`.
- `stacks`, `removes`, `removes_behaviours`, `removes_activities`, `settings`, `add`, `schedule` and `targets` refine how it combines with the girl's other traits.

### A small example

A trait that makes a girl wander slowly whenever she is awake and free:

```json
{
  "category": "territory",
  "description": "Strolls about at an easy pace.",
  "params": {
    "speed": { "type": "number", "default": 0.6, "min": 0.1, "max": 4.0, "description": "How fast she strolls." }
  },
  "goals": [
    { "priority": 5, "goal": "wander", "when": "free_and_awake", "speed": "$speed" }
  ]
}
```

Each goal has a `priority` (lower runs first), the `goal` id, an optional `when` condition, and that goal's own settings. A setting can be a fixed value or a `$name` taken from `params`. The Goals page lists each goal's settings.

## Conditions

A goal can carry a condition so it only runs sometimes: `tamed`, `untamed`, `not_sleeping`, `free_and_awake`, `following_and_tamed` and `following_tamed_and_awake`.

## Goals and behaviours

Wild girls run **goals**. Settlers and Kobolds run a **brain** of behaviours, which is why the reference has both lists. A trait can add to either, but the behaviours are written in the game, so a pack chooses among them rather than writing new ones.

## Common problems

- **An unknown goal**: the id is misspelt, or it is a brain behaviour rather than a goal.
- **A `$name` that is not declared**: add it to `params`.
- **Two traits fighting**: only categories marked "several" can hold more than one.
