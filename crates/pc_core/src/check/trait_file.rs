//! A trait file: a piece of how a girl behaves, written in the goals the game has.

use super::values::{check_value, quoted, unknown_key};
use crate::diagnostics::Sink;
use crate::json_ast::{Node, Value};
use crate::reference::{nearest, qualified, short, Field, Reference};

pub fn check(sink: &mut Sink, root: &Node, reference: &Reference) {
    let Some(members) = root.as_object() else {
        sink.error("wrong-type", format!("A trait file must be an object ({{ ... }}), not {}.", root.describe()), root.start, root.end);
        return;
    };

    let keys: Vec<&str> = reference.trait_file_keys.iter().map(String::as_str).collect();

    for member in members {
        if !keys.contains(&member.key.as_str()) {
            unknown_key(sink, member, keys.iter().copied(), "a trait");
        }
    }

    match root.get("category") {
        None => sink.error("missing-field", format!("Add a \"category\": one of {}.", reference.categories.keys().cloned().collect::<Vec<_>>().join(", ")), root.start, root.start + 1),
        Some(member) => match member.value.as_str() {
            Some(name) if reference.categories.contains_key(name) => {}
            Some(name) => {
                let fix = nearest(name, reference.categories.keys().map(String::as_str));
                let message = format!("\"{name}\" is not a category. Use one of: {}.", reference.categories.keys().cloned().collect::<Vec<_>>().join(", "));

                match fix {
                    Some(fix) => sink.error_with_fix("bad-choice", message, member.value.start, member.value.end, format!("Change to \"{fix}\""), quoted(&fix)),
                    None => sink.error("bad-choice", message, member.value.start, member.value.end),
                }
            }
            None => sink.error("wrong-type", "\"category\" must be text.", member.value.start, member.value.end),
        },
    }

    let params = params(sink, root);

    for key in ["goals", "targets"] {
        if let Some(member) = root.get(key) {
            goals(sink, &member.value, key, reference, &params);
        }
    }

    if let Some(member) = root.get("capabilities") {
        capabilities(sink, &member.value, reference, &params);
    }

    if let Some(member) = root.get("stacks") {
        match member.value.as_array() {
            Some(items) => {
                for item in items {
                    if !matches!(item.as_str(), Some("goals") | Some("brain")) {
                        sink.error("bad-choice", "A stack is \"goals\" (wild girls) or \"brain\" (settlers).", item.start, item.end);
                    }
                }
            }
            None => sink.error("wrong-type", "\"stacks\" must be a list: [\"goals\"], [\"brain\"] or both.", member.value.start, member.value.end),
        }
    }
}

/// The settings the trait declares, by name. Each is `{ "type": "number", "default": 1, "min": 0, "max": 4, "description": "..." }`.
fn params(sink: &mut Sink, root: &Node) -> Vec<String> {
    let Some(member) = root.get("params") else { return Vec::new() };
    let Some(entries) = member.value.as_object() else {
        sink.error("wrong-type", "\"params\" must be an object with one entry for each setting.", member.value.start, member.value.end);
        return Vec::new();
    };

    for entry in entries {
        let Some(body) = entry.value.as_object() else {
            sink.error("wrong-type", format!("The setting \"{}\" must be an object with a \"default\".", entry.key), entry.value.start, entry.value.end);
            continue;
        };

        if !body.iter().any(|m| m.key == "default") {
            sink.error("missing-field", format!("The setting \"{}\" needs a \"default\".", entry.key), entry.value.start, entry.value.start + 1);
        }

        if let Some(kind) = entry.value.get("type") {
            if !matches!(kind.value.as_str(), Some("number") | Some("integer") | Some("boolean") | Some("identifier")) {
                sink.error("bad-choice", "A setting's \"type\" is number, integer, boolean or identifier.", kind.value.start, kind.value.end);
            }
        }
    }

    entries.iter().map(|entry| entry.key.clone()).collect()
}

fn goals(sink: &mut Sink, node: &Node, key: &str, reference: &Reference, params: &[String]) {
    let Some(items) = node.as_array() else {
        sink.error("wrong-type", format!("\"{key}\" must be a list of goals."), node.start, node.end);
        return;
    };

    for item in items {
        let Some(entries) = item.as_object() else {
            sink.error("wrong-type", "A goal is an object with a \"priority\" and a \"goal\".", item.start, item.end);
            continue;
        };

        match item.get("goal").and_then(|m| m.value.as_str().map(|name| (m, name))) {
            None => sink.error("missing-field", "A goal needs a \"goal\": its name.", item.start, item.start + 1),
            Some((member, name)) => match reference.goals.get(&qualified(name)) {
                Some(fields) => goal_settings(sink, entries, fields, name, params),
                None => {
                    let fix = nearest(name, reference.goals.keys().map(|k| short(k)));
                    let message = format!("There is no goal called \"{name}\".{}", fix.as_ref().map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default());

                    match fix {
                        Some(fix) => sink.error_with_fix("unknown-goal", message, member.value.start, member.value.end, format!("Change to \"{fix}\""), quoted(&fix)),
                        None => sink.error("unknown-goal", message, member.value.start, member.value.end),
                    }
                }
            },
        }

        match item.get("priority") {
            None => sink.error("missing-field", "A goal needs a \"priority\": a whole number (the lower, the sooner).", item.start, item.start + 1),
            Some(member) if !matches!(member.value.value, Value::Number { .. }) => sink.error("wrong-type", "\"priority\" must be a number.", member.value.start, member.value.end),
            Some(_) => {}
        }

        if let Some(member) = item.get("when") {
            match member.value.as_str() {
                Some(name) if reference.goal_conditions.iter().any(|c| c == &qualified(name)) => {}
                Some(name) => {
                    let fix = nearest(name, reference.goal_conditions.iter().map(|c| short(c)));

                    sink.error("unknown-condition", format!("There is no condition called \"{name}\".{}", fix.map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default()), member.value.start, member.value.end);
                }
                None => sink.error("wrong-type", "\"when\" must be a condition's name.", member.value.start, member.value.end),
            }
        }
    }
}

fn goal_settings(sink: &mut Sink, entries: &[crate::json_ast::Member], fields: &[Field], goal: &str, params: &[String]) {
    for entry in entries.iter().filter(|e| !matches!(e.key.as_str(), "goal" | "priority" | "when")) {
        match fields.iter().find(|f| f.name == entry.key) {
            None => unknown_key(sink, entry, fields.iter().map(|f| f.name.as_str()), &format!("the goal \"{goal}\"")),
            Some(field) => match entry.value.as_str() {
                /* "$name" stands for a setting of the trait. */
                Some(reference) if reference.starts_with('$') => reference_exists(sink, &entry.value, reference, params),
                _ => check_value(sink, field, &entry.value),
            },
        }
    }
}

fn capabilities(sink: &mut Sink, node: &Node, reference: &Reference, params: &[String]) {
    let Some(entries) = node.as_object() else {
        sink.error("wrong-type", "\"capabilities\" must be an object.", node.start, node.end);
        return;
    };

    for entry in entries {
        match reference.capabilities.get(&entry.key) {
            None => unknown_key(sink, entry, reference.capabilities.keys().map(String::as_str), "capabilities"),
            Some(field) => match entry.value.as_str() {
                Some(text) if text.starts_with('$') => reference_exists(sink, &entry.value, text, params),
                _ => check_value(sink, field, &entry.value),
            },
        }
    }
}

fn reference_exists(sink: &mut Sink, node: &Node, text: &str, params: &[String]) {
    if !params.iter().any(|p| p == &text[1..]) {
        sink.error("unknown-setting", format!("\"{text}\" names a setting this trait does not have. Add \"{}\" under \"params\".", &text[1..]), node.start, node.end);
    }
}
