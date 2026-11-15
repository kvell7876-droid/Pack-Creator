//! JSON Schemas for the editor, made from the reference. The editor's JSON support turns them into completion, hover help and squiggles as the user types; the checks in
//! [`crate::check`] then do what a schema cannot (what other files of the pack have, what the game would refuse).

use crate::layout::FileKind;
use crate::reference::{short, Field, Reference};
use serde_json::{json, Map, Value};

fn field_schema(field: &Field) -> Value {
    let mut schema = match field.kind.as_str() {
        "boolean" => json!({ "type": "boolean" }),
        "integer" => json!({ "type": "integer" }),
        "number" => json!({ "type": "number" }),
        "colour" => json!({ "type": "string", "pattern": "^#[0-9a-fA-F]{6}$" }),
        "item" | "identifier" => json!({ "type": "string", "pattern": "^([a-z0-9_.-]+:)?[a-z0-9_./-]*$" }),
        "vec3" => json!({ "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 }),
        "choice" | "movement" if !field.options.is_empty() => json!({ "enum": field.options }),
        "list" => json!({ "type": "array", "items": if field.options.is_empty() { json!({ "type": "string" }) } else { json!({ "enum": field.options }) } }),
        "object" if matches!(field.default, Some(Value::Array(_))) => json!({ "type": "array", "items": { "type": "string" } }),
        "object" => json!({}),
        _ => json!({ "type": "string" }),
    };

    let map = schema.as_object_mut().unwrap();

    if let Some(min) = field.min {
        map.insert("minimum".into(), json!(min));
    }

    if let Some(max) = field.max {
        map.insert("maximum".into(), json!(max));
    }

    if !field.description.is_empty() {
        map.insert("description".into(), json!(field.description));
    }

    if let Some(default) = &field.default {
        map.insert("default".into(), default.clone());
    }

    schema
}

fn object_schema(fields: &[Field], extra: &[(&str, Value)]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for field in fields {
        properties.insert(field.name.clone(), field_schema(field));

        if field.required {
            required.push(field.name.clone());
        }
    }

    for (name, schema) in extra {
        properties.insert((*name).to_string(), schema.clone());
    }

    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

/// The schema of a girl file. `pack_traits` are the traits the pack itself defines, offered beside the game's.
pub fn girl(reference: &Reference, pack_traits: &[String]) -> Value {
    let mut components = Map::new();

    for (id, component) in &reference.components {
        let schema = if short(id) == "ai" { ai(reference, pack_traits) } else { object_schema(&component.fields, &[]) };
        let mut schema = schema;

        schema.as_object_mut().unwrap().insert("description".into(), json!(component.description));
        components.insert(id.clone(), schema);
    }

    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "pattern": "^[a-z0-9_]+$", "description": "A short name in lower case letters, digits and underscores. It should match the file's name." },
            "type": { "enum": reference.archetype_names(), "description": "What kind of girl she is: a settler lives in a settlement, a wild girl is met out in the world, a merchant travels and trades." },
            "components": { "type": "object", "properties": components, "additionalProperties": false, "description": "The parts she is made of." }
        },
        "required": ["id", "type", "components"],
        "additionalProperties": false
    })
}

fn ai(reference: &Reference, pack_traits: &[String]) -> Value {
    let mut categories = Map::new();

    for (name, category) in &reference.categories {
        let mut names: Vec<String> = reference.traits.values().filter(|t| &t.category == name).map(|t| short(&t.id).to_string()).collect();

        names.extend(pack_traits.iter().cloned());
        names.sort();
        names.dedup();

        let one = json!({ "anyOf": [ { "enum": names }, { "type": "object", "properties": { "type": { "enum": names } }, "required": ["type"] } ] });
        let schema = if category.several { json!({ "anyOf": [one.clone(), { "type": "array", "items": one }] }) } else { one };
        let mut schema = schema;

        schema.as_object_mut().unwrap().insert("description".into(), json!(category.description));
        categories.insert(name.clone(), schema);
    }

    json!({
        "type": "object",
        "properties": {
            "preset": { "type": "string", "description": "The brain preset her traits start from. Leave it out for the standard one." },
            "traits": { "type": "object", "properties": categories, "additionalProperties": false, "description": "How she behaves, one trait for each category." }
        },
        "additionalProperties": false
    })
}

pub fn scene(reference: &Reference) -> Value {
    let override_fields: Vec<Field> = reference.scene_fields.iter().filter(|f| !matches!(f.name.as_str(), "supported_girls" | "unsupported_girls" | "girl_overrides")).cloned().collect();
    let overrides = json!({ "type": "object", "additionalProperties": object_schema(&override_fields, &[]), "description": "Settings that change for one girl, by her id." });

    let fields: Vec<Field> = reference.scene_fields.iter().filter(|f| f.name != "girl_overrides").cloned().collect();

    object_schema(&fields, &[("girl_overrides", overrides)])
}

pub fn trait_file(reference: &Reference) -> Value {
    let goal_names: Vec<String> = reference.goals.keys().map(|g| short(g).to_string()).collect();
    let conditions: Vec<String> = reference.goal_conditions.iter().map(|c| short(c).to_string()).collect();
    let goal = json!({
        "type": "object",
        "properties": {
            "priority": { "type": "integer", "description": "The lower, the sooner she does it." },
            "goal": { "enum": goal_names, "description": "What she does." },
            "when": { "enum": conditions, "description": "Only while this is true." }
        },
        "required": ["priority", "goal"]
    });
    let capability_names: Vec<&String> = reference.capabilities.keys().collect();

    json!({
        "type": "object",
        "properties": {
            "category": { "enum": reference.categories.keys().collect::<Vec<_>>(), "description": "The kind of choice this trait is." },
            "description": { "type": "string", "description": "What it does, for tools." },
            "params": { "type": "object", "description": "The settings a girl may tune. Use them in a goal as \"$name\".", "additionalProperties": {
                "type": "object", "properties": {
                    "type": { "enum": ["number", "integer", "boolean", "identifier"] }, "default": {}, "min": { "type": "number" }, "max": { "type": "number" }, "description": { "type": "string" } },
                "required": ["default"] } },
            "goals": { "type": "array", "items": goal.clone(), "description": "The goals it adds for a wild girl." },
            "targets": { "type": "array", "items": goal, "description": "The target goals it adds (who she picks to fight)." },
            "removes": { "type": "array", "items": { "type": "string" }, "description": "Goals it takes out of the whole plan." },
            "capabilities": { "type": "object", "properties": reference.capabilities.iter().map(|(name, field)| (name.clone(), json!({ "description": field.description }))).collect::<Map<_, _>>(),
                "description": format!("What it changes about her body: {}.", capability_names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", ")) },
            "stacks": { "type": "array", "items": { "enum": ["goals", "brain"] }, "description": "The kinds of girl it is for." },
            "removes_behaviours": { "type": "array", "items": { "type": "string" } },
            "removes_activities": { "type": "array", "items": { "type": "string" } },
            "settings": { "type": "object" },
            "add": { "type": "array" },
            "schedule": { "type": "object" }
        },
        "required": ["category"],
        "additionalProperties": false
    })
}

/// A dialogue file: a list of lines.
pub fn dialogue(reference: &Reference) -> Value {
    let tags = json!({ "type": "array", "items": { "enum": reference.taste_tags }, "description": "What the line appeals to: she likes or dislikes it by her taste." });
    let reply = |what: &str| json!({ "type": "string", "description": what });
    let answer = json!({
        "type": "object",
        "properties": { "text": { "type": "string", "description": "What the player says." }, "tags": tags.clone(), "reply_good": reply("How she answers when it lands."), "reply_bad": reply("How she answers when it does not.") },
        "required": ["text", "reply_good", "reply_bad"],
        "additionalProperties": false
    });

    json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "pattern": "^[a-z0-9_]+$", "description": "The line's id, unique across dialogue." },
                "topic": { "enum": reference.dialogue_topics, "description": "What the conversation is about." },
                "tags": tags,
                "player_line": { "type": "string", "description": "What is said (what the player says, or what she asks when the line has answers)." },
                "reply_good": reply("Her reply when it lands."),
                "reply_bad": reply("Her reply when it does not."),
                "unsupported_girls": { "type": "array", "items": { "type": "string" }, "description": "Girls this line is not offered to." },
                "answers": { "type": "array", "items": answer, "description": "Makes it a line she opens: the player picks one of these." },
                "requires_flow": { "type": "integer", "description": "How well the conversation must be going." },
                "requires_affection": { "type": "integer", "minimum": 0, "maximum": 10, "description": "The affection she needs." },
                "requires_said": { "type": "string", "enum": reference.dialogue_ids, "description": "A line that must have been said before (a follow-up)." },
                "once": { "type": "boolean", "description": "Offered only once, ever." },
                "weight": { "type": "integer", "minimum": 1, "description": "How likely it is to be picked." }
            },
            "required": ["id", "topic", "player_line"],
            "additionalProperties": false
        }
    })
}

/// A small talk file: a list of entries (or one).
pub fn social(reference: &Reference) -> Value {
    let one_or_many = |options: Vec<String>, what: &str| json!({ "anyOf": [{ "enum": options }, { "type": "array", "items": { "enum": options } }], "description": what });
    let entry = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "pattern": "^[a-z0-9_]+$", "description": "A name for the entry." },
            "kind": { "enum": crate::social::KINDS, "description": "line: said between two settlers. offer: said to the player when a settler on break walks over." },
            "text": { "type": "string", "description": "What is said. {name}, {partner} and {interest} are filled in." },
            "outcome": one_or_many(crate::social::strings(&crate::social::OUTCOMES), "How the chat is going."),
            "personality": one_or_many(crate::social::strings(&crate::social::PERSONALITIES), "The speaker's personality."),
            "job": one_or_many(crate::social::strings(&crate::social::JOBS), "The speaker's job."),
            "mood": one_or_many(crate::social::strings(&crate::social::MOODS), "The speaker's mood."),
            "time": { "enum": crate::social::TIMES, "description": "Day or night." },
            "weather": { "enum": crate::social::WEATHERS, "description": "The weather." },
            "interest": { "enum": reference.taste_tags, "description": "Only when the pair both like this interest. It teaches the player that they do." },
            "role": { "enum": crate::social::ROLES, "description": "Who in the pair says it: the one who began the chat, or the one who answers." },
            "weight": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "How likely it is against the others that fit." }
        },
        "required": ["text"],
        "additionalProperties": false
    });

    json!({ "anyOf": [{ "type": "array", "items": entry.clone() }, entry] })
}

/// The schema for a kind of file, or None when there is none (the editor then checks only that it is JSON).
pub fn for_kind(kind: &FileKind, reference: &Reference, pack_traits: &[String]) -> Option<Value> {
    match kind {
        FileKind::Girl => Some(girl(reference, pack_traits)),
        FileKind::Scene => Some(scene(reference)),
        FileKind::Trait => Some(trait_file(reference)),
        FileKind::Dialogue => Some(dialogue(reference)),
        FileKind::Social => Some(social(reference)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_girl_schema_lists_the_components_and_the_traits() {
        let reference = Reference::builtin();
        let schema = girl(&reference, &["shy_fox".to_string()]);
        let body = &schema["properties"]["components"]["properties"]["pleasurehorizons:body"];

        assert_eq!(body["properties"]["body_type"]["enum"][0], "jenny");
        assert_eq!(body["properties"]["held_item_offset"]["maxItems"], 3);
        assert!(body["required"].as_array().unwrap().iter().any(|v| v == "body_type"));

        let movement = &schema["properties"]["components"]["properties"]["pleasurehorizons:ai"]["properties"]["traits"]["properties"]["movement"];

        assert!(movement.to_string().contains("walker"));
        assert!(movement.to_string().contains("shy_fox"));
    }

    #[test]
    fn every_kind_with_a_schema_makes_one() {
        let reference = Reference::builtin();

        for kind in [FileKind::Girl, FileKind::Scene, FileKind::Trait] {
            assert!(for_kind(&kind, &reference, &[]).is_some());
        }

        assert!(for_kind(&FileKind::Dialogue, &reference, &[]).is_some());
        assert!(for_kind(&FileKind::Lang, &reference, &[]).is_none());
    }
}
