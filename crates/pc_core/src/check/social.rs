//! Small talk files: what settlers say on a break. A file is a list of entries (or one entry), each a line with the conditions it needs.

use super::values::{check_value, unknown_key};
use crate::diagnostics::Sink;
use crate::json_ast::Node;
use crate::reference::{Field, Reference};
use crate::social;

fn choice(name: &str, options: Vec<String>, list: bool) -> Field {
    Field { name: name.into(), kind: if list { "list" } else { "choice" }.into(), required: false, description: String::new(), min: None, max: None, options, default: None, of: None }
}

pub fn check(sink: &mut Sink, root: &Node, reference: &Reference) {
    let items: &[Node] = match root.as_array() {
        Some(items) => items,
        None if root.as_object().is_some() => std::slice::from_ref(root),
        None => {
            sink.error("wrong-type", format!("A small talk file is a list ([ ... ]) of entries, not {}.", root.describe()), root.start, root.end);
            return;
        }
    };

    let mut ids: Vec<&str> = Vec::new();

    for item in items {
        let Some(members) = item.as_object() else {
            sink.error("wrong-type", "Each entry is an object with a \"text\" and any conditions.", item.start, item.end);
            continue;
        };

        for member in members {
            if !social::KEYS.contains(&member.key.as_str()) {
                unknown_key(sink, member, social::KEYS.iter().copied(), "a small talk entry");
            }
        }

        match item.get("text").map(|m| (m, m.value.as_str())) {
            None => sink.error("missing-field", "Each entry needs a \"text\": what is said.", item.start, item.start + 1),
            Some((member, None)) => sink.error("wrong-type", "\"text\" must be text.", member.value.start, member.value.end),
            Some((member, Some(text))) if text.trim().is_empty() => sink.error("empty-text", "\"text\" is empty.", member.value.start, member.value.end),
            Some((member, Some(text))) => placeholders(sink, text, member.value.start, member.value.end),
        }

        if let Some(member) = item.get("id") {
            match member.value.as_str() {
                Some(id) => {
                    if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                        sink.error("bad-id", format!("\"{id}\" is not a valid id: use lower case letters, digits and underscores."), member.value.start, member.value.end);
                    }

                    if ids.contains(&id) {
                        sink.warn("duplicate-id", format!("The id \"{id}\" is used by another entry in this file."), member.value.start, member.value.end);
                    }

                    ids.push(id);
                }
                None => sink.error("wrong-type", "\"id\" must be text.", member.value.start, member.value.end),
            }
        }

        let lists = [("outcome", social::strings(&social::OUTCOMES)), ("personality", social::strings(&social::PERSONALITIES)), ("job", social::strings(&social::JOBS)), ("mood", social::strings(&social::MOODS))];

        for (key, options) in lists {
            if let Some(member) = item.get(key) {
                /* The game takes one value or a list of them. */
                check_value(sink, &choice(key, options, member.value.as_array().is_some()), &member.value);
            }
        }

        let singles = [("kind", social::strings(&social::KINDS)), ("time", social::strings(&social::TIMES)), ("weather", social::strings(&social::WEATHERS)), ("role", social::strings(&social::ROLES)), ("interest", reference.taste_tags.clone())];

        for (key, options) in singles {
            if let Some(member) = item.get(key) {
                if options.is_empty() {
                    continue;
                }

                check_value(sink, &choice(key, options, false), &member.value);
            }
        }

        if let Some(member) = item.get("weight") {
            let field = Field { name: "weight".into(), kind: "integer".into(), required: false, description: String::new(), min: Some(1.0), max: Some(1000.0), options: Vec::new(), default: None, of: None };

            check_value(sink, &field, &member.value);
        }

        /* An offer is said once, to the player, so a pair's interest and who speaks first have no meaning for it. */
        let is_offer = item.get("kind").and_then(|m| m.value.as_str()) == Some("offer");

        if is_offer {
            for key in ["interest", "role"] {
                if let Some(member) = item.get(key) {
                    sink.warn("ignored-condition", format!("An offer is said to the player, not between two settlers, so \"{key}\" never matches and the offer will not be used."), member.value.start, member.value.end);
                }
            }
        }
    }
}

/// Warns about a `{word}` in a text that the game does not fill in.
fn placeholders(sink: &mut Sink, text: &str, start: usize, end: usize) {
    let mut rest = text;

    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else { break };
        let word = &rest[open + 1..open + close];

        if !social::PLACEHOLDERS.contains(&word) {
            sink.warn("bad-placeholder", format!("\"{{{word}}}\" is not filled in by the game and will be said as it is. Use {}.", social::PLACEHOLDERS.iter().map(|p| format!("{{{p}}}")).collect::<Vec<_>>().join(", ")), start, end);
        }

        rest = &rest[open + close + 1..];
    }
}
