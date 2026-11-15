//! The small file kinds: pack.mcmeta, dialogue and language files.

use super::values::{check_value, quoted, unknown_key};
use super::PackFacts;
use crate::diagnostics::Sink;
use crate::json_ast::{Node, Value};
use crate::reference::{nearest, Field, Reference};

/// The data pack format of Minecraft 1.21.1.
pub const PACK_FORMAT: i64 = 48;

pub fn mcmeta(sink: &mut Sink, root: &Node) {
    let Some(pack) = root.get("pack") else {
        sink.error("missing-field", "pack.mcmeta needs a \"pack\" section with a pack_format and a description.", root.start, root.start + 1);
        return;
    };

    match pack.value.get("pack_format") {
        None => sink.error("missing-field", format!("Add \"pack_format\": {PACK_FORMAT} (the data pack format of Minecraft 1.21.1)."), pack.value.start, pack.value.start + 1),
        Some(member) => match &member.value.value {
            Value::Number { value, .. } if *value as i64 == PACK_FORMAT => {}
            Value::Number { value, .. } => sink.warn("pack-format", format!("pack_format {} is not the format of Minecraft 1.21.1, which is {PACK_FORMAT}. The game may refuse the pack or show a warning.", *value as i64), member.value.start, member.value.end),
            _ => sink.error("wrong-type", "\"pack_format\" must be a number.", member.value.start, member.value.end),
        },
    }

    if pack.value.get("description").is_none() {
        sink.warn("missing-field", "Add a \"description\": what the pack is, shown in the pack list.", pack.value.start, pack.value.start + 1);
    }
}

/// The reactions a line or option may name; the game plays `react_<name>` on her. The same list as `GirlReactions.ALL` in the mod.
const REACTIONS: [&str; 16] = ["nod", "shake_head", "smile", "laugh", "blush", "shy", "surprised", "sad", "angry", "pout", "think", "shrug", "excited", "sigh", "flirt", "bow"];

const NODE_TYPES: [&str; 7] = ["line", "choice", "pool", "branch", "random", "effect", "end"];

/// A node's name must exist in the graph.
fn check_target(sink: &mut Sink, names: &[&str], holder: &Node, key: &str) {
    let Some(member) = holder.get(key) else { return };

    match member.value.as_str() {
        Some(target) if names.contains(&target) => {}
        Some(target) => {
            let fix = nearest(target, names.iter().copied());
            sink.error("unknown-node", format!("There is no node called \"{target}\" in this graph.{}", fix.map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default()), member.value.start, member.value.end);
        }
        None => sink.error("wrong-type", format!("\"{key}\" is the name of a node."), member.value.start, member.value.end),
    }
}

/// A reaction must be one the game has a clip name for.
fn check_reaction(sink: &mut Sink, holder: &Node) {
    let Some(member) = holder.get("reaction") else { return };

    match member.value.as_str() {
        Some("") => {}
        Some(name) if REACTIONS.contains(&name) => {}
        Some(name) => {
            let fix = nearest(name, REACTIONS.iter().copied());
            sink.error("unknown-reaction", format!("\"{name}\" is not a reaction. Use one of: {}.{}", REACTIONS.join(", "), fix.map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default()), member.value.start, member.value.end);
        }
        None => sink.error("wrong-type", "\"reaction\" is the name of a reaction.", member.value.start, member.value.end),
    }
}

/// A graph file: nodes by name, a start, and links between them that must all lead somewhere.
fn graph(sink: &mut Sink, root: &Node) {
    let Some(nodes) = root.get("nodes").and_then(|m| m.value.as_object()) else {
        sink.error("wrong-type", "\"nodes\" is an object of nodes by name.", root.start, root.start + 1);
        return;
    };

    let names: Vec<&str> = nodes.iter().map(|m| m.key.as_str()).collect();

    match root.get("start") {
        None => sink.error("missing-field", "A graph needs a \"start\": the name of the node it begins at.", root.start, root.start + 1),
        Some(member) => match member.value.as_str() {
            Some(start) if names.contains(&start) => {}
            Some(start) => sink.error("unknown-node", format!("\"start\" is \"{start}\", which is not a node of this graph."), member.value.start, member.value.end),
            None => sink.error("wrong-type", "\"start\" is the name of a node.", member.value.start, member.value.end),
        },
    }

    for node in nodes {
        let holder = &node.value;

        if holder.as_object().is_none() {
            sink.error("wrong-type", format!("The node \"{}\" must be an object with a \"type\".", node.key), holder.start, holder.end);
            continue;
        }

        match holder.get("type") {
            None => sink.error("missing-field", format!("The node \"{}\" needs a \"type\": {}.", node.key, NODE_TYPES.join(", ")), holder.start, holder.start + 1),
            Some(member) => match member.value.as_str() {
                Some(kind) if NODE_TYPES.contains(&kind) => {}
                Some(kind) => sink.error("bad-choice", format!("\"{kind}\" is not a node type. Use one of: {}.", NODE_TYPES.join(", ")), member.value.start, member.value.end),
                None => sink.error("wrong-type", "\"type\" is text.", member.value.start, member.value.end),
            },
        }

        check_target(sink, &names, holder, "next");
        check_target(sink, &names, holder, "else");
        check_reaction(sink, holder);

        for (list, link) in [("options", "next"), ("branches", "to"), ("ways", "to"), ("lines", "next")] {
            let Some(items) = holder.get(list).and_then(|m| m.value.as_array()) else { continue };

            for item in items {
                check_target(sink, &names, item, link);
                check_reaction(sink, item);
            }
        }
    }
}

/// A bark file: a pool of short lines, or a list of pools. Each line needs its text.
fn barks(sink: &mut Sink, root: &Node) {
    let pools: Vec<&Node> = match root.as_array() {
        Some(items) => items.iter().collect(),
        None => vec![root],
    };

    for pool in pools {
        if pool.get("pool").and_then(|m| m.value.as_str()).is_none() {
            sink.error("missing-field", "Each bark pool needs a \"pool\" name, such as \"gift_refused\".", pool.start, pool.start + 1);
        }

        let Some(lines) = pool.get("lines").and_then(|m| m.value.as_array()) else {
            sink.error("missing-field", "Each bark pool needs a list of \"lines\".", pool.start, pool.start + 1);
            continue;
        };

        for line in lines {
            if line.get("text").and_then(|m| m.value.as_str()).is_none() {
                sink.error("missing-field", "Each line needs its \"text\".", line.start, line.start + 1);
            }

            for member in line.as_object().unwrap_or(&[]) {
                if !["text", "when", "weight", "girls"].contains(&member.key.as_str()) {
                    unknown_key(sink, member, ["text", "when", "weight", "girls"], "a bark line");
                }
            }
        }
    }
}

/// A dialogue file is a list of lines (small talk), a graph (dialogue), or a pool of barks. Small talk lines are each one thing the player can say (or, with `answers`, one
/// thing she asks and the player answers), with the taste tags it carries and what it needs of the moment.
pub fn dialogue(sink: &mut Sink, root: &Node, reference: &Reference, facts: &PackFacts) {
    if root.as_object().is_some() && root.get("nodes").is_some() {
        graph(sink, root);
        return;
    }

    let bark_shaped = root.get("pool").is_some() || root.as_array().and_then(|items| items.first()).is_some_and(|first| first.get("pool").is_some());

    if bark_shaped {
        barks(sink, root);
        return;
    }

    let Some(items) = root.as_array() else {
        sink.error("wrong-type", format!("A dialogue file is a list ([ ... ]) of lines, not {}.", root.describe()), root.start, root.end);
        return;
    };

    const KEYS: [&str; 12] = ["id", "topic", "tags", "player_line", "reply_good", "reply_bad", "unsupported_girls", "answers", "requires_flow", "requires_affection", "requires_said", "once"];
    let keys: Vec<&str> = KEYS.iter().copied().chain(["weight"]).collect();
    let tag_field = Field { name: "tags".into(), kind: "list".into(), required: false, description: String::new(), min: None, max: None, options: reference.taste_tags.clone(), default: None, of: None };
    let mut ids: Vec<&str> = Vec::new();

    for item in items {
        let Some(members) = item.as_object() else {
            sink.error("wrong-type", "Each entry is an object with an \"id\", a \"topic\" and the lines.", item.start, item.end);
            continue;
        };

        for member in members {
            if !keys.contains(&member.key.as_str()) {
                unknown_key(sink, member, keys.iter().copied(), "a dialogue line");
            }
        }

        match item.get("id").and_then(|m| m.value.as_str().map(|id| (m, id))) {
            None => sink.error("missing-field", "Each line needs an \"id\".", item.start, item.start + 1),
            Some((member, id)) => {
                if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                    sink.error("bad-id", format!("\"{id}\" is not a valid line id: use lower case letters, digits and underscores."), member.value.start, member.value.end);
                }

                if ids.contains(&id) {
                    sink.error("duplicate-id", format!("The id \"{id}\" is used by another line in this file."), member.value.start, member.value.end);
                } else if reference.dialogue_ids.contains(id) {
                    sink.warn("duplicate-id", format!("The game already has a line called \"{id}\"; this one may replace it or be ignored."), member.value.start, member.value.end);
                }

                ids.push(id);
            }
        }

        match item.get("topic") {
            None => sink.error("missing-field", format!("Each line needs a \"topic\": {}.", reference.dialogue_topics.join(", ")), item.start, item.start + 1),
            Some(member) => match member.value.as_str() {
                Some(topic) if reference.dialogue_topics.iter().any(|t| t == topic) => {}
                Some(topic) => {
                    let fix = nearest(topic, reference.dialogue_topics.iter().map(String::as_str));
                    let message = format!("\"{topic}\" is not a topic. Use one of: {}.{}", reference.dialogue_topics.join(", "), fix.as_ref().map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default());

                    match fix {
                        Some(fix) => sink.error_with_fix("bad-choice", message, member.value.start, member.value.end, format!("Change to \"{fix}\""), quoted(&fix)),
                        None => sink.error("bad-choice", message, member.value.start, member.value.end),
                    }
                }
                None => sink.error("wrong-type", "\"topic\" must be text.", member.value.start, member.value.end),
            },
        }

        match item.get("player_line").map(|m| (m, m.value.as_str())) {
            None => sink.error("missing-field", "Each line needs a \"player_line\": what is said.", item.start, item.start + 1),
            Some((member, None)) => sink.error("wrong-type", "\"player_line\" must be text.", member.value.start, member.value.end),
            Some((member, Some(text))) if text.trim().is_empty() => sink.error("empty-text", "\"player_line\" is empty.", member.value.start, member.value.end),
            Some(_) => {}
        }

        let has_answers = item.get("answers").map_or(false, |m| m.value.as_array().map_or(false, |a| !a.is_empty()));

        for key in ["reply_good", "reply_bad"] {
            if let Some(member) = item.get(key) {
                if member.value.as_str().is_none() {
                    sink.error("wrong-type", format!("\"{key}\" must be text."), member.value.start, member.value.end);
                }
            }
        }

        if !has_answers && item.get("reply_good").is_none() && item.get("reply_bad").is_none() {
            sink.warn("no-reply", "This line has no reply_good or reply_bad (and no answers), so she says nothing back.", item.start, item.start + 1);
        }

        if let Some(member) = item.get("tags") {
            check_value(sink, &tag_field, &member.value);
        }

        if let Some(member) = item.get("unsupported_girls") {
            match member.value.as_array() {
                Some(list) => list.iter().filter(|g| g.as_str().is_none()).for_each(|g| sink.error("wrong-type", "A girl id is text.", g.start, g.end)),
                None => sink.error("wrong-type", "\"unsupported_girls\" must be a list of girl ids.", member.value.start, member.value.end),
            }
        }

        /* Affection stops at ten, so a line that asks for more can never be offered: a mistake, but one the game lives with. */
        if let Some(member) = item.get("requires_affection") {
            if member.value.as_f64().map_or(false, |n| n > 10.0) {
                sink.warn("unreachable-line", "Affection stops at 10, so this line can never be offered.", member.value.start, member.value.end);
            }
        }

        for (key, min, max) in [("requires_affection", 0.0, f64::INFINITY), ("requires_flow", -100.0, 100.0), ("weight", 1.0, 1000.0)] {
            if let Some(member) = item.get(key) {
                let field = Field { name: key.into(), kind: "integer".into(), required: false, description: String::new(), min: Some(min), max: Some(max), options: Vec::new(), default: None, of: None };

                check_value(sink, &field, &member.value);
            }
        }

        if let Some(member) = item.get("once") {
            if !matches!(member.value.value, Value::Bool(_)) {
                sink.error("wrong-type", "\"once\" must be true or false.", member.value.start, member.value.end);
            }
        }

        if let Some(member) = item.get("requires_said") {
            match member.value.as_str() {
                Some(id) if facts.dialogue_ids.contains(id) || reference.dialogue_ids.contains(id) => {}
                Some(id) => {
                    let known = facts.dialogue_ids.iter().chain(reference.dialogue_ids.iter()).map(String::as_str);
                    let fix = nearest(id, known);

                    sink.error("unknown-line", format!("There is no dialogue line called \"{id}\" (in this pack or the game).{}", fix.map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default()), member.value.start, member.value.end);
                }
                None => sink.error("wrong-type", "\"requires_said\" is the id of a line.", member.value.start, member.value.end),
            }
        }

        if let Some(member) = item.get("answers") {
            let Some(answers) = member.value.as_array() else {
                sink.error("wrong-type", "\"answers\" must be a list of answers.", member.value.start, member.value.end);
                continue;
            };

            for answer in answers {
                let Some(fields) = answer.as_object() else {
                    sink.error("wrong-type", "An answer is an object with text, tags, reply_good and reply_bad.", answer.start, answer.end);
                    continue;
                };

                for field in fields {
                    if !["text", "tags", "reply_good", "reply_bad"].contains(&field.key.as_str()) {
                        unknown_key(sink, field, ["text", "tags", "reply_good", "reply_bad"], "an answer");
                    }
                }

                for key in ["text", "reply_good", "reply_bad"] {
                    if answer.get(key).and_then(|m| m.value.as_str()).is_none() {
                        sink.error("missing-field", format!("Each answer needs \"{key}\" (text)."), answer.start, answer.start + 1);
                    }
                }

                if let Some(tags) = answer.get("tags") {
                    check_value(sink, &tag_field, &tags.value);
                }
            }
        }
    }
}

/// A language file maps keys to text.
pub fn lang(sink: &mut Sink, root: &Node) {
    let Some(members) = root.as_object() else {
        sink.error("wrong-type", "A language file is an object of keys and text.", root.start, root.end);
        return;
    };

    for member in members {
        if member.value.as_str().is_none() {
            sink.error("wrong-type", format!("\"{}\" must be text.", member.key), member.value.start, member.value.end);
        }
    }
}
