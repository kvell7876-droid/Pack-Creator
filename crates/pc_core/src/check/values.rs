//! Checking one value against what the reference says a field is, and an object against a list of fields. The girl, scene and trait checks are built from these.

use crate::diagnostics::Sink;
use crate::json_ast::{Member, Node, Value};
use crate::reference::{nearest, Field};

/// Whether `text` is a Minecraft style id: `namespace:path` or `path`, lower case.
pub fn is_identifier(text: &str) -> bool {
    let valid = |part: &str, slash: bool| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.') || (slash && c == '/'));

    match text.split_once(':') {
        Some((namespace, path)) => valid(namespace, false) && valid(path, true),
        None => valid(text, true),
    }
}

/// The text of a key or a string as it is written in a file, quotes included (what a repair puts in place of the old one).
pub fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

/// Checks that `node` is a right value for `field`.
pub fn check_value(sink: &mut Sink, field: &Field, node: &Node) {
    let name = &field.name;
    let wrong_kind = |sink: &mut Sink, expected: &str| {
        sink.error("wrong-type", format!("\"{name}\" must be {expected}, not {}.", node.describe()), node.start, node.end);
    };

    match field.kind.as_str() {
        "boolean" => {
            if !matches!(node.value, Value::Bool(_)) {
                wrong_kind(sink, "true or false");
            }
        }
        "integer" | "number" => match &node.value {
            Value::Number { value, integer } => {
                if field.kind == "integer" && !integer && value.fract() != 0.0 {
                    sink.error("not-whole", format!("\"{name}\" must be a whole number."), node.start, node.end);
                }

                range(sink, field, *value, node);
            }
            _ => wrong_kind(sink, "a number"),
        },
        "text" => {
            if node.as_str().is_none() {
                wrong_kind(sink, "text");
            }
        }
        "colour" => match node.as_str() {
            Some(text) => {
                let hex = text.strip_prefix('#').unwrap_or(text);

                if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    sink.error("bad-colour", format!("\"{text}\" is not a colour. Write it as #RRGGBB, for example #FFAA00."), node.start, node.end);
                }
            }
            None => wrong_kind(sink, "a colour such as \"#FFAA00\""),
        },
        "choice" | "movement" => match node.as_str() {
            Some(text) => choice(sink, field, text, node),
            None => wrong_kind(sink, "one of its options"),
        },
        "item" | "identifier" => match node.as_str() {
            Some("") if field.kind == "identifier" => {}
            Some(text) if is_identifier(text) => {}
            Some(text) => sink.error("bad-id", format!("\"{text}\" is not a valid id. Write it like minecraft:allium: lower case letters, digits and _ - . only."), node.start, node.end),
            None => wrong_kind(sink, "an id such as minecraft:allium"),
        },
        "vec3" => match node.as_array() {
            Some(items) if items.len() == 3 && items.iter().all(|item| item.as_f64().is_some()) => {}
            Some(_) => sink.error("bad-vec3", format!("\"{name}\" needs exactly three numbers: [x, y, z]."), node.start, node.end),
            None => wrong_kind(sink, "three numbers: [x, y, z]"),
        },
        "list" | "object" => list_or_object(sink, field, node),
        _ => {}
    }
}

fn list_or_object(sink: &mut Sink, field: &Field, node: &Node) {
    let list_like = field.kind == "list" || matches!(field.default, Some(serde_json::Value::Array(_)));

    if !list_like {
        return;
    }

    let Some(items) = node.as_array() else {
        sink.error("wrong-type", format!("\"{}\" must be a list.", field.name), node.start, node.end);
        return;
    };

    for item in items {
        match item.as_str() {
            Some(text) if !field.options.is_empty() && !field.options.iter().any(|option| option == text) => {
                let fix = nearest(text, field.options.iter().map(String::as_str));
                let message = format!("\"{text}\" is not one of: {}.{}", field.options.join(", "), fix.as_ref().map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default());

                match fix {
                    Some(fix) => sink.error_with_fix("bad-choice", message, item.start, item.end, format!("Change to \"{fix}\""), quoted(&fix)),
                    None => sink.error("bad-choice", message, item.start, item.end),
                }
            }
            Some(_) => {}
            None => sink.error("wrong-type", format!("Every entry of \"{}\" must be text.", field.name), item.start, item.end),
        }
    }
}

fn choice(sink: &mut Sink, field: &Field, text: &str, node: &Node) {
    if field.options.is_empty() || field.options.iter().any(|option| option == text) {
        return;
    }

    let fix = nearest(text, field.options.iter().map(String::as_str));
    let message = format!("\"{text}\" is not a choice for \"{}\". Use one of: {}.{}", field.name, field.options.join(", "), fix.as_ref().map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default());

    match fix {
        Some(fix) => sink.error_with_fix("bad-choice", message, node.start, node.end, format!("Change to \"{fix}\""), quoted(&fix)),
        None => sink.error("bad-choice", message, node.start, node.end),
    }
}

fn range(sink: &mut Sink, field: &Field, value: f64, node: &Node) {
    if let Some(min) = field.min {
        if value < min - 1e-6 {
            sink.error("too-small", format!("\"{}\" cannot be less than {}.", field.name, trim(min)), node.start, node.end);
            return;
        }
    }

    if let Some(max) = field.max {
        if value > max + 1e-6 {
            sink.error("too-large", format!("\"{}\" cannot be more than {}.", field.name, trim(max)), node.start, node.end);
        }
    }
}

fn trim(number: f64) -> String {
    if number.fract() == 0.0 { format!("{}", number as i64) } else { format!("{:.4}", number).trim_end_matches('0').to_string() }
}

/// Checks the members of an object against `fields`: an unknown name, a name written twice, a wrong value, a missing required one.
/// `skip` names members the caller checks itself (they are not reported as unknown and not checked here).
pub fn check_object(sink: &mut Sink, object: &Node, fields: &[Field], what: &str, skip: &[&str]) {
    let Some(members) = object.as_object() else {
        sink.error("wrong-type", format!("{what} must be an object ({{ ... }}), not {}.", object.describe()), object.start, object.end);
        return;
    };

    let mut seen: Vec<&str> = Vec::new();

    for member in members {
        if seen.contains(&member.key.as_str()) {
            sink.error("duplicate-key", format!("\"{}\" is written twice in {what}.", member.key), member.key_start, member.key_end);
            continue;
        }

        seen.push(&member.key);

        if skip.contains(&member.key.as_str()) {
            continue;
        }

        match fields.iter().find(|field| field.name == member.key) {
            Some(field) => check_value(sink, field, &member.value),
            None => unknown_key(sink, member, fields.iter().map(|field| field.name.as_str()), what),
        }
    }

    for field in fields.iter().filter(|field| field.required) {
        if !members.iter().any(|member| member.key == field.name) {
            sink.error("missing-field", format!("{what} needs \"{}\": {}", field.name, field.description), object.start, object.start + 1);
        }
    }
}

/// Reports a name that is not one of `known`, with the nearest as a repair.
pub fn unknown_key<'a>(sink: &mut Sink, member: &Member, known: impl IntoIterator<Item = &'a str> + Clone, what: &str) {
    let list: Vec<&str> = known.clone().into_iter().collect();
    let fix = nearest(&member.key, known);
    let message = format!("\"{}\" is not a setting of {what}.{}", member.key, match &fix {
        Some(fix) => format!(" Did you mean \"{fix}\"?"),
        None if list.is_empty() => String::new(),
        None => format!(" It has: {}.", list.join(", ")),
    });

    match fix {
        Some(fix) => sink.error_with_fix("unknown-field", message, member.key_start, member.key_end, format!("Change to \"{fix}\""), quoted(&fix)),
        None => sink.error("unknown-field", message, member.key_start, member.key_end),
    }
}
