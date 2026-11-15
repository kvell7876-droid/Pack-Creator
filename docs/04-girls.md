# Girl files

`data/<namespace>/girls/<id>.json`. The id is lower case letters, digits and underscores (`amber`). The smallest girl:

```json
{
  "id": "amber",
  "type": "settler",
  "components": {
    "pleasurehorizons:identity": { "name": "Amber" },
    "pleasurehorizons:body": { "body_type": "jenny", "hitbox_height": 1.8 },
    "pleasurehorizons:taste": { "tame_item": "minecraft:allium" }
  }
}
```

The girl form writes this for you. Every field, its range and its default is in [Girl components](ref:components); this page explains how they fit together.

## Type

The `type` picks what kind of girl she is, and with it the components she must have:

- `settler`: lives in a settlement, works a job, can be recruited by contract.
- `wild`: found in the world, follows a gift she likes, can be tamed.
- `merchant`: visits settlements to trade. Merchants are found by visiting, so their tame item does not have to be unique.

All three need `identity`, `body` and `taste`. The checks tell you which one is missing.

## Components

Her file is a set of **components**, each a block of settings named like `pleasurehorizons:body`. A field you leave out takes its default. The ones you will use most:

| Component | What it sets |
|---|---|
| `identity` | Her name, and the two colours of her spawn egg |
| `body` | Her skeleton (`body_type`), height, how a held item sits in her hand, flat lighting, whether she refuses single beds |
| `stats` | Health, speed (0.2 is a walking player), attack damage, and how large her belly can grow |
| `taste` | The `tame_item` she follows and accepts. Each wild girl should have her own |
| `personality` | Her archetype: `shy`, `bold`, `motherly`, `bratty` or `stoic`. It biases the likes and dislikes she is rolled once |
| `dialogue` | The lines she says when given a gift she likes or loves |
| `ai` | Her traits, and the brain preset they start from |
| `spawn` | Where and when a wild girl appears by herself |
| `merchant` | How often a merchant visits, how she wanders, and how she dresses as trust grows |

### Body, and the held item

`hitbox_height` is her height in blocks. If a sword or tool sits wrong in her hand, nudge it with `held_item_offset` (model pixels), `held_item_rotation` (degrees) and `held_item_scale`. Those are added to where the game holds it, so zero is the game's default, not "nothing".

`shading` chooses how she is lit: `soft`, `vanilla` (the game's own two lights), `toon` (flat tones with a clean edge) or `flat` (no side darker than another), or `default` to use the setting the player chose in the game's config. Use `flat` when the texture already has shading painted in and the game's lighting makes one side too dark.

### Spawning

`weight` is her chance against the other wild girls who fit the place; **0 means she never spawns on her own** (spawn egg and shrine only). `biomes` takes ids (`minecraft:swamp`) or tags (`#minecraft:is_forest`); empty means anywhere. `time` is `any`, `day` or `night`. `group_min`, `group_max` and `max_nearby` stop a village of one girl from becoming a crowd.

## Skeleton (body type)

`body_type` is the base model her animations were made for. The game's skeletons are `jenny`, `bia`, `ellie`, `luna`, `goblin`, `galath`, `kobold` and `copper_golem`. The skeleton decides which scenes fit her: a scene lists the body types it works with. Several girls can share one, and a scene's animation can be written once for a whole skeleton ([scene animations](doc:animations)).

A skeleton is a promise about **bones**. If your model's bones differ from the skeleton's, the scene animations will not drive her correctly. The new girl wizard checks this for you.

## Her files

A girl's own model, texture, animations and cues are always looked up in the `pleasurehorizons` namespace, whatever namespace your data uses. The checks warn if you put them anywhere else. The whole set is described in [assets](doc:assets).

## Traits

A girl's behaviour comes from traits, grouped in categories: movement, temperament, combat, loadout, interaction, routine, territory and work. She has **one** trait in each category that allows only one, and may have several in `interaction`. A category she leaves out gets its default, so a girl with no traits at all still walks, defends herself, sleeps and roams. See [trait files](doc:traits) and the [Traits](ref:traits) reference.

## Common problems

- **"missing required component"**: add `identity`, `body` and `taste`.
- **A model, texture or animation warning**: the file is missing or in the wrong namespace.
- **Two wild girls with one tame item**: she cannot be told apart. Give each her own.
- **She spawns everywhere or nowhere**: check `weight` and `biomes` in `spawn`.
