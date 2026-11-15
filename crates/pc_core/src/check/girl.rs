//! A girl file: `{ "id": ..., "type": ..., "components": { ... } }`.

use super::values::{check_object, check_value, quoted, unknown_key};
use super::PackFacts;
use crate::diagnostics::Sink;
use crate::json_ast::Node;
use crate::reference::{nearest, qualified, short, Reference};

pub fn check(sink: &mut Sink, root: &Node, stem: &str, reference: &Reference, facts: &PackFacts) {
    let Some(members) = root.as_object() else {
        sink.error("wrong-type", format!("A girl file must be an object ({{ ... }}), not {}.", root.describe()), root.start, root.end);
        return;
    };

    for member in members {
        if !["id", "type", "components"].contains(&member.key.as_str()) {
            sink.warn("unknown-key", format!("\"{}\" is ignored. A girl file has: id, type, components.", member.key), member.key_start, member.key_end);
        }
    }

    match root.get("id") {
        None => sink.error("missing-field", "Add an \"id\": a short name in lower case letters, digits and underscores, such as \"lucy\".", root.start, root.start + 1),
        Some(member) => match member.value.as_str() {
            Some(id) if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') => {
                sink.error("bad-id", format!("\"{id}\" is not a valid id. Use lower case letters, digits and underscores only."), member.value.start, member.value.end)
            }
            Some(id) if id != stem => sink.warn("id-differs-from-file", format!("The id \"{id}\" is not the file's name \"{stem}\". The game uses the id, but the two should match."), member.value.start, member.value.end),
            Some(_) => {}
            None => sink.error("wrong-type", "\"id\" must be text.", member.value.start, member.value.end),
        },
    }

    let archetype = archetype(sink, root, reference);

    let Some(components) = root.get("components") else {
        sink.error("missing-field", "Add \"components\": the parts she is made of (body, identity, taste and so on).", root.start, root.start + 1);
        return;
    };

    let Some(list) = components.value.as_object() else {
        sink.error("wrong-type", "\"components\" must be an object.", components.value.start, components.value.end);
        return;
    };

    let mut written: Vec<String> = Vec::new();

    for member in list {
        let id = qualified(&member.key);

        if written.contains(&id) {
            sink.error("duplicate-key", format!("The component \"{}\" is written twice.", member.key), member.key_start, member.key_end);
            continue;
        }

        written.push(id.clone());

        let Some(component) = reference.components.get(&id) else {
            let fix = nearest(&member.key, reference.components.keys().map(|k| if member.key.contains(':') { k.as_str() } else { short(k) }));
            let message = format!("There is no component called \"{}\".{}", member.key, fix.as_ref().map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_else(|| format!(" Components: {}.", reference.components.keys().map(|k| short(k)).collect::<Vec<_>>().join(", "))));

            match fix {
                Some(fix) => sink.error_with_fix("unknown-component", message, member.key_start, member.key_end, format!("Change to \"{fix}\""), quoted(&fix)),
                None => sink.error("unknown-component", message, member.key_start, member.key_end),
            }

            continue;
        };

        if short(&id) == "ai" {
            ai(sink, &member.value, reference, facts);
        } else {
            check_object(sink, &member.value, &component.fields, &format!("the {} component", short(&id)), &[]);
        }
    }

    if let Some(needs) = archetype.and_then(|a| reference.archetypes.get(&a)) {
        for needed in needs {
            if !written.contains(needed) {
                sink.error("missing-component", format!("This kind of girl needs the \"{}\" component: {}", short(needed), reference.components.get(needed).map(|c| c.description.as_str()).unwrap_or("")), components.value.start, components.value.start + 1);
            }
        }
    }
}

/// Checks `type` and gives back the archetype's full id when it is a known one.
fn archetype(sink: &mut Sink, root: &Node, reference: &Reference) -> Option<String> {
    let Some(member) = root.get("type") else {
        sink.error("missing-field", format!("Add a \"type\": one of {}.", reference.archetype_names().join(", ")), root.start, root.start + 1);
        return None;
    };

    let Some(text) = member.value.as_str() else {
        sink.error("wrong-type", "\"type\" must be text.", member.value.start, member.value.end);
        return None;
    };

    let id = qualified(text);

    if reference.archetypes.contains_key(&id) {
        return Some(id);
    }

    let names = reference.archetype_names();
    let fix = nearest(text, names.iter().map(String::as_str));
    let message = format!("\"{text}\" is not a kind of girl. Use one of: {}.{}", names.join(", "), fix.as_ref().map(|f| format!(" Did you mean \"{f}\"?")).unwrap_or_default());

    match fix {
        Some(fix) => sink.error_with_fix("unknown-type", message, member.value.start, member.value.end, format!("Change to \"{fix}\""), quoted(&fix)),
        None => sink.error("unknown-type", message, member.value.start, member.value.end),
    }

    None
}

/// The ai component: `{ "preset": "", "traits": { "<category>": <trait> } }`, a trait being a name, an object with a `type` or, where a category takes several, a list.
fn ai(sink: &mut Sink, node: &Node, reference: &Reference, facts: &PackFacts) {
    let Some(members) = node.as_object() else {
        sink.error("wrong-type", "The ai component must be an object.", node.start, node.end);
        return;
    };

    for member in members {
        match member.key.as_str() {
            "preset" => {
                if member.value.as_str().is_none() {
                    sink.error("wrong-type", "\"preset\" must be text.", member.value.start, member.value.end);
                }
            }
            "traits" => traits(sink, &member.value, reference, facts),
            _ => unknown_key(sink, member, ["preset", "traits"], "the ai component"),
        }
    }
}

fn traits(sink: &mut Sink, node: &Node, reference: &Reference, facts: &PackFacts) {
    let Some(members) = node.as_object() else {
        sink.error("wrong-type", "\"traits\" must be an object with one entry for each category.", node.start, node.end);
        return;
    };

    for member in members {
        let Some(category) = reference.categories.get(&member.key) else {
            unknown_key(sink, member, reference.categories.keys().map(String::as_str), "traits");
            continue;
        };

        match member.value.as_array() {
            Some(items) if category.several => items.iter().for_each(|item| one_trait(sink, item, &member.key, reference, facts)),
            Some(_) => sink.error("wrong-type", format!("\"{}\" takes one trait, not a list.", member.key), member.value.start, member.value.end),
            None => one_trait(sink, &member.value, &member.key, reference, facts),
        }
    }
}

fn one_trait(sink: &mut Sink, node: &Node, category: &str, reference: &Reference, facts: &PackFacts) {
    let (name_node, settings) = match (node.as_str(), node.as_object()) {
        (Some(_), _) => (node.clone(), None),
        (_, Some(_)) => match node.get("type") {
            Some(member) => (member.value.clone(), Some(node)),
            None => {
                sink.error("missing-field", "A trait written as an object needs a \"type\": its name.", node.start, node.start + 1);
                return;
            }
        },
        _ => {
            sink.error("wrong-type", "A trait is its name (\"walker\") or an object with a \"type\".", node.start, node.end);
            return;
        }
    };

    let Some(name) = name_node.as_str() else {
        sink.error("wrong-type", "\"type\" must be the trait's name.", name_node.start, name_node.end);
        return;
    };

    let id = qualified(name);
    let theirs = facts.traits.get(&id);
    let ours = reference.traits.get(&id);

    if theirs.is_none() && ours.is_none() {
        let candidates: Vec<&str> = reference.traits.values().filter(|t| t.category == category).map(|t| short(&t.id)).chain(facts.traits.keys().map(|k| short(k))).collect();
        let fix = nearest(name, candidates.iter().copied());
        let message = format!("There is no trait called \"{name}\" for {category}.{}", match &fix {
            Some(fix) => format!(" Did you mean \"{fix}\"?"),
            None => format!(" Traits for {category}: {}.", candidates.join(", ")),
        });

        match fix {
            Some(fix) => sink.error_with_fix("unknown-trait", message, name_node.start, name_node.end, format!("Change to \"{fix}\""), quoted(&fix)),
            None => sink.error("unknown-trait", message, name_node.start, name_node.end),
        }

        return;
    }

    if let Some(known) = ours {
        if known.category != category {
            sink.error("wrong-category", format!("\"{name}\" is a trait for {}, not {category}.", known.category), name_node.start, name_node.end);
        }
    }

    if let Some(members) = settings.and_then(|s| s.as_object()) {
        let params: Vec<&str> = match (ours, theirs) {
            (_, Some(own)) => own.iter().map(String::as_str).collect(),
            (Some(known), None) => known.params.iter().map(|p| p.name.as_str()).collect(),
            _ => Vec::new(),
        };

        for member in members.iter().filter(|member| member.key != "type") {
            match ours.and_then(|known| known.params.iter().find(|p| p.name == member.key)) {
                Some(field) => check_value(sink, field, &member.value),
                None if params.contains(&member.key.as_str()) => {}
                None => unknown_key(sink, member, params.iter().copied(), &format!("the trait \"{name}\"")),
            }
        }

    }

}
