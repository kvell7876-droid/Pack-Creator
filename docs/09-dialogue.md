# Dialogue files

`data/<namespace>/dialogue/<name>.json` is a list of exchanges. In the game, a player talks to a girl by choosing a topic and a line to say; she answers well or badly depending on how the line suits her taste.

```json
[
  {
    "id": "first_line",
    "topic": "chat",
    "tags": [],
    "player_line": "What the player says.",
    "reply_good": "How she answers when it goes well.",
    "reply_bad": "How she answers when it does not."
  }
]
```

## The fields

| Field | Does |
|---|---|
| `id` | Names the exchange. Lower case, unique |
| `topic` | The kind of conversation (below) |
| `tags` | What the exchange appeals to; a girl who likes the tag answers well |
| `player_line` | What the player says |
| `reply_good` | How she answers when it goes well |
| `reply_bad` | How she answers when it does not |

The text is written out literally, not as a language key. Optional keys: `requires_affection`, `requires_said`, `requires_flow`, `once`, `weight`, `answers` and `unsupported_girls`.

## Topics

`chat`, `compliment`, `flirt`, `joke`, `about_her` and `about_village`. The player picks the topic first; the game then offers lines of that topic.

## Making a conversation unfold

- `requires_said` holds an exchange back until another has been said, so a story runs in order. The ids the game's own lines use are in [Dialogue](ref:dialogue); you can follow one of them.
- `requires_affection` keeps a line until she likes the player enough.
- `once` says it only ever happens once.
- `answers` makes the line one **she opens**: she says `player_line` and the player picks how to respond from the answers.
- `weight` makes a line more or less likely to be offered. The default is 1.
- `requires_flow` keeps a line until the conversation's flow, which rises as it goes well, has reached that number.
- `unsupported_girls` keeps a line away from girls it does not suit.

## Tags and taste

A girl's likes and dislikes are rolled once from her personality, and `tags` is how a line matches them. Using the right tags is what makes a shy girl and a bold girl answer the same line differently. The tags are the same as for scenes ([scene settings](ref:scene)).

## Writing good lines

- Write the line in the girl's voice. A line is read out of context, so avoid "as I said".
- Give `reply_bad` a real answer. A girl who is only ever pleased feels flat.
- Short lines read better in the chat box than long ones.

## Small talk on breaks

Make one with **Ctrl+N, Small talk**: it opens as a form with a chip for every condition the game understands, and the checks name any word the game would not know (a misspelt job, a weather that does not exist, a `{placeholder}` that is not filled in). Settlers on break chat with each other and offer to talk to the player. Those lines are data too: files in `data/<namespace>/social/`, a list of entries like

```json
{
  "id": "farmer_harvest",
  "kind": "line",
  "text": "The harvest looks good this year, {partner}.",
  "job": ["farmer"],
  "outcome": ["warm", "easy"],
  "weather": "clear"
}
```

`kind` is `line` (between two settlers) or `offer` (to the player, when a settler on break walks over). Every other key is a condition, and leaving one out matches anything: `outcome` (`warm`, `easy`, `flat`, `clash`), `personality`, `job`, `mood` (`happy`, `content`, `tired`, `upset`, `afraid`), `time` (`day`, `night`), `weather` (`clear`, `rain`, `thunder`), `interest` (a taste tag the pair both like; hearing it teaches the player that they like it), `role` (`starter` or `answerer`) and `weight`. The line is chosen at random among those that fit, and the more conditions an entry has the likelier it is, so a line about the rain beats a generic one when it is raining. `{name}`, `{partner}` and `{interest}` in the text are filled in.

## Gift reactions

What she says when given a gift she likes or loves is not here: it is the `gift_like` and `gift_love` lists in her `pleasurehorizons:dialogue` component ([girl files](doc:girls)).

## Common problems

- **A line that never appears**: its `requires_said` points at an id that does not exist, or its `requires_affection` is too high.
- **Two lines with one id**: the second is ignored. Ids are unique across the whole game, so prefix yours with the pack's name.
