//! What the game says it has: the components a girl is made of and their fields, the traits and their settings, the fields of a scene. Everything the checks and the
//! editor's suggestions know about the game comes from here, so when the game gains a setting a new `reference.json` is all the tool needs.
//!
//! The file is written by the game (`/pleasure dev girlSchema`) and shipped inside the program. A pack's own traits are added on top of it by the caller.

use crate::layout::GAME_NAMESPACE;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const BUILTIN: &str = include_str!("../reference/reference.json");

/// One setting of a component, a trait or a scene.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    /// boolean, integer, number, text, colour, choice, item, vec3, list, object (and identifier, for a trait's settings).
    pub kind: String,
    pub required: bool,
    pub description: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Vec<String>,
    pub default: Option<Value>,
    /// For a list: what its items are (`body_type`, `taste_tag`, `girl_id`), when the reference says.
    pub of: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Component {
    pub id: String,
    pub description: String,
    pub required: bool,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitRef {
    pub id: String,
    pub category: String,
    pub description: String,
    pub params: Vec<Field>,
    pub stacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub name: String,
    pub description: String,
    pub several: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reference {
    pub components: BTreeMap<String, Component>,
    /// Archetype id to the components it needs (`pleasurehorizons:wild` to body, identity and taste).
    pub archetypes: BTreeMap<String, Vec<String>>,
    pub traits: BTreeMap<String, TraitRef>,
    pub categories: BTreeMap<String, Category>,
    pub scene_fields: Vec<Field>,
    pub taste_tags: Vec<String>,
    pub trait_file_keys: Vec<String>,
    /// The goals a trait may use, by id, and the conditions a goal may carry.
    pub goals: BTreeMap<String, Vec<Field>>,
    pub goal_conditions: Vec<String>,
    /// The capabilities a trait may set, by name.
    pub capabilities: BTreeMap<String, Field>,
    /// The sound events the game has (full ids), and the cue names its own animations fire.
    pub sound_events: BTreeSet<String>,
    pub known_cues: BTreeSet<String>,
    /// The topics a dialogue line can be about, and the ids of the game's own lines (a line can follow one with `requires_said`).
    pub dialogue_topics: Vec<String>,
    pub dialogue_ids: BTreeSet<String>,
}

/// Writes an id with its namespace: a bare `body` means the game's.
pub fn qualified(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else {
        format!("{GAME_NAMESPACE}:{id}")
    }
}

/// The part of an id after its namespace.
pub fn short(id: &str) -> &str {
    id.rsplit_once(':').map(|(_, name)| name).unwrap_or(id)
}

impl Reference {
    /// The reference that ships with the program.
    pub fn builtin() -> Reference {
        Reference::from_json(BUILTIN).expect("the shipped reference is valid")
    }

    pub fn from_json(text: &str) -> Result<Reference, String> {
        let root: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut reference = Reference {
            components: BTreeMap::new(),
            archetypes: BTreeMap::new(),
            traits: BTreeMap::new(),
            categories: BTreeMap::new(),
            scene_fields: Vec::new(),
            taste_tags: Vec::new(),
            trait_file_keys: Vec::new(),
            goals: BTreeMap::new(),
            goal_conditions: Vec::new(),
            capabilities: BTreeMap::new(),
            sound_events: BTreeSet::new(),
            known_cues: BTreeSet::new(),
            dialogue_topics: Vec::new(),
            dialogue_ids: BTreeSet::new(),
        };

        for (id, body) in object(root.get("components")) {
            reference.components.insert(id.clone(), Component {
                id: id.clone(),
                description: string(body.get("description")),
                required: body.get("required").and_then(Value::as_bool).unwrap_or(false),
                fields: fields(body.get("fields")),
            });
        }

        for (id, needs) in object(root.get("archetypes")) {
            reference.archetypes.insert(id.clone(), needs.as_array().map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default());
        }

        if let Some(ai) = root.get("ai") {
            for (name, body) in object(ai.get("trait_categories")) {
                reference.categories.insert(name.clone(), Category {
                    name: name.clone(),
                    description: string(body.get("description")),
                    several: body.get("several").and_then(Value::as_bool).unwrap_or(false),
                });
            }

            for (id, body) in object(ai.get("traits")) {
                reference.traits.insert(id.clone(), TraitRef {
                    id: id.clone(),
                    category: string(body.get("category")),
                    description: string(body.get("description")),
                    params: fields(body.get("params")),
                    stacks: body.get("stacks").and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                });
            }
        }

        if let Some(ai) = root.get("ai") {
            for (id, body) in object(ai.get("goals")) {
                reference.goals.insert(id.clone(), fields(body.get("fields")));
            }

            reference.goal_conditions = object(ai.get("goal_conditions")).into_iter().map(|(id, _)| id.clone()).collect();

            for (name, body) in object(ai.get("trait_capabilities")) {
                reference.capabilities.insert(name.clone(), Field {
                    name: name.clone(),
                    kind: string(body.get("kind")),
                    required: false,
                    description: string(body.get("description")),
                    min: None,
                    max: None,
                    options: body.get("options").and_then(Value::as_array).map(|o| o.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                    default: None,
                    of: None,
                });
            }
        }

        if let Some(scene) = root.get("scene") {
            reference.scene_fields = fields(scene.get("fields"));
            reference.taste_tags = scene.get("taste_tags").and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        }

        reference.sound_events = root.get("sound_events").and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        reference.known_cues = root.get("cues").and_then(|c| c.get("known")).and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();

        if let Some(dialogue) = root.get("dialogue") {
            reference.dialogue_topics = dialogue.get("topics").and_then(Value::as_array).map(|l| l.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
            reference.dialogue_ids = dialogue.get("ids").and_then(Value::as_array).map(|l| l.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        }

        let body_types = reference.body_types();
        let taste_tags = reference.taste_tags.clone();

        for field in &mut reference.scene_fields {
            match field.of.as_deref() {
                Some("body_type") => field.options = body_types.clone(),
                Some("taste_tag") => field.options = taste_tags.clone(),
                _ => {}
            }
        }

        if let Some(keys) = root.get("trait_file").and_then(|t| t.get("keys")).and_then(Value::as_array) {
            reference.trait_file_keys = keys.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }

        Ok(reference)
    }

    /// The component an id (written with or without its namespace) names.
    pub fn component(&self, id: &str) -> Option<&Component> {
        self.components.get(&qualified(id))
    }

    /// The body types the game has, as the body component lists them.
    pub fn body_types(&self) -> Vec<String> {
        self.component("body")
            .and_then(|body| body.fields.iter().find(|field| field.name == "body_type"))
            .map(|field| field.options.clone())
            .unwrap_or_default()
    }

    /// The archetypes a girl file may name, without their namespace (`settler`, `wild`, `merchant`).
    pub fn archetype_names(&self) -> Vec<String> {
        self.archetypes.keys().map(|id| short(id).to_string()).collect()
    }

    pub fn trait_ref(&self, id: &str) -> Option<&TraitRef> {
        self.traits.get(&qualified(id))
    }
}

fn object(value: Option<&Value>) -> Vec<(&String, &Value)> {
    value.and_then(Value::as_object).map(|map| map.iter().collect()).unwrap_or_default()
}

fn string(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or("").to_string()
}

fn fields(value: Option<&Value>) -> Vec<Field> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|field| Field {
                    name: string(field.get("name")),
                    kind: string(field.get("kind")),
                    required: field.get("required").and_then(Value::as_bool).unwrap_or(false),
                    description: string(field.get("description")),
                    min: field.get("min").and_then(Value::as_f64),
                    max: field.get("max").and_then(Value::as_f64),
                    options: field.get("options").and_then(Value::as_array).map(|o| o.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                    default: field.get("default").cloned(),
                    of: field.get("of").and_then(Value::as_str).map(String::from),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The names in `options` that are closest to `wrong`, for "did you mean". Empty when nothing is close enough.
pub fn nearest<'a>(wrong: &str, options: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let wrong = wrong.to_ascii_lowercase();
    let mut best: Option<(usize, String)> = None;

    for option in options {
        let distance = edit_distance(&wrong, &option.to_ascii_lowercase());
        let limit = (option.len().max(wrong.len()) / 3).max(2);

        if distance <= limit && best.as_ref().map_or(true, |(d, _)| distance < *d) {
            best = Some((distance, option.to_string()));
        }
    }

    best.map(|(_, name)| name)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();

    for i in 1..=a.len() {
        let mut last = row[0];

        row[0] = i;

        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let next = (row[j] + 1).min(row[j - 1] + 1).min(last + cost);

            last = row[j];
            row[j] = next;
        }
    }

    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_reference_reads() {
        let reference = Reference::builtin();

        assert!(reference.component("body").is_some());
        assert!(reference.component("pleasurehorizons:body").is_some());
        assert!(reference.body_types().contains(&"jenny".to_string()));
        assert_eq!(reference.archetype_names(), vec!["merchant", "settler", "wild"]);
        assert!(reference.trait_ref("walker").is_some());
        assert!(reference.trait_ref("stationary").is_some());
        assert!(reference.categories["interaction"].several);
        assert!(!reference.scene_fields.is_empty());
        assert!(reference.goals.contains_key("pleasurehorizons:wander"));
        assert!(reference.goal_conditions.contains(&"pleasurehorizons:untamed".to_string()));
        assert!(reference.capabilities.contains_key("movement"));
        assert!(reference.sound_events.contains("pleasurehorizons:kobold.moan"));
        assert!(reference.known_cues.contains("thrust"));
        assert!(reference.scene_fields.iter().find(|f| f.name == "body_types").unwrap().options.contains(&"ellie".to_string()));
    }

    #[test]
    fn suggests_the_nearest_name() {
        assert_eq!(nearest("jeny", ["jenny", "bia", "ellie"]), Some("jenny".to_string()));
        assert_eq!(nearest("settlr", ["settler", "wild"]), Some("settler".to_string()));
        assert_eq!(nearest("zzzzzz", ["settler", "wild"]), None);
    }
}
