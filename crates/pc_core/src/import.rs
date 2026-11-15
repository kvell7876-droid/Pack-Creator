//! Bringing in an old girl. The old profile (a flat file with her name, colours, hitbox, attributes and a list of scenes) becomes a girl file of components and one scene
//! file for each of her old scenes, with a report of what was carried over, changed and left behind.
//!
//! Her models, textures, animations and sounds are a separate part of the job (they are converted by Strut's converter) and are not done here.

use crate::error::{Error, Result};
use crate::reference::Reference;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Carried over as it was.
    Kept,
    /// Carried over in a changed form.
    Changed,
    /// Left behind, with the reason.
    Dropped,
    /// Something to look at.
    Review,
    /// Something that will not work until it is put right.
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub level: Level,
    pub message: String,
}

/// One scene of an old profile, and the old animations it played (each with the part it plays: `slow`, `fast`, `cum`...).
#[derive(Debug, Clone, Serialize)]
pub struct ImportedScene {
    pub id: String,
    pub old_name: String,
    /// `(role, old animation name)`: a role listed more than once gets `_2`, `_3`... after it.
    pub roles: Vec<(String, String)>,
}

/// What importing a profile gives: the files to write (relative to the pack) and the report.
#[derive(Debug, Serialize)]
pub struct Imported {
    pub girl_id: String,
    pub scenes: Vec<ImportedScene>,
    /// The girl's own file, `data/<namespace>/girls/<id>.json`, and her scenes' files.
    pub files: Vec<(String, String)>,
    /// Language entries (scene names) to merge into the pack's `en_us.json`.
    pub lang: BTreeMap<String, String>,
    pub notes: Vec<Note>,
}

/// Scene ids the game ships: an imported scene of the same name is called `<id>_custom` so it does not replace the game's own.
pub const BUILTIN_SCENES: [&str; 10] = ["anal", "blow_job", "cowgirl", "doggy", "face_fuck", "masturbation", "mating_press", "missionary", "paizuri", "standing_sex"];

/// The animation fields of an old scene and the part each plays.
const ROLE_FIELDS: [(&str, &str); 9] = [
    ("intro_anim", "intro"), ("slow_anim", "slow"), ("fast_anim", "fast"), ("cum_anim", "cum"), ("lay_on_bed_anim", "lay_on_bed"), ("lay_down_anim", "lay_on_bed"),
    ("bed_idle_anim", "bed_idle"), ("idle_anim", "bed_idle"), ("anim", "loop"),
];

fn names_of(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
        Some(Value::Array(list)) => list.iter().filter_map(|v| v.as_str().filter(|s| !s.is_empty()).map(String::from)).collect(),
        _ => Vec::new(),
    }
}

fn roles_of(scene: &Value, kind: &str) -> Vec<(String, String)> {
    let mut roles = Vec::new();

    for (field, role) in ROLE_FIELDS {
        if scene.get(field).is_none() {
            continue;
        }

        let role = if role == "intro" && kind == "stationary_intro" { "solo_intro" } else { role };

        for (i, name) in names_of(scene.get(field)).into_iter().enumerate() {
            roles.push((if i == 0 { role.to_string() } else { format!("{role}_{}", i + 1) }, name));
        }
    }

    roles
}

fn slug(text: &str) -> String {
    let mapped: String = text.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();

    mapped.split('_').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("_")
}

/// Reads an old profile. `fallback_id` is used when the file has no `id` (the file's name). `existing_scenes` are scene ids the pack already has.
pub fn import_profile(text: &str, fallback_id: &str, body_type: &str, namespace: &str, existing_scenes: &[String], reference: &Reference) -> Result<Imported> {
    let profile: Value = serde_json::from_str(text).map_err(|e| Error::invalid(format!("The profile is not valid JSON: {e}")))?;
    let Some(old) = profile.as_object() else {
        return Err(Error::invalid("The profile must be a JSON object."));
    };

    if old.contains_key("components") {
        return Err(Error::invalid("This is already a girl in the new format (it has components). Open it in the pack instead of importing it."));
    }

    if !reference.body_types().iter().any(|b| b == body_type) {
        return Err(Error::invalid(format!("\"{body_type}\" is not a body type. Choose one of: {}.", reference.body_types().join(", "))));
    }

    let mut notes = Vec::new();
    let mut note = |level: Level, message: String| notes.push(Note { level, message });

    let id = slug(old.get("id").and_then(Value::as_str).unwrap_or(fallback_id));

    if id.is_empty() {
        return Err(Error::invalid("The profile has no id and its file has no usable name."));
    }

    let name = old.get("name").and_then(Value::as_str).unwrap_or(&id).to_string();
    let mut identity = Map::new();

    identity.insert("name".into(), json!(name));

    for (from, to) in [("primary", "egg_primary"), ("secondary", "egg_secondary")] {
        if let Some(colour) = old.get("egg_color").and_then(|c| c.get(from)).and_then(Value::as_str) {
            identity.insert(to.into(), json!(colour));
        }
    }

    let mut body = Map::new();
    let own_body = old.get("body_type").and_then(Value::as_str).filter(|b| reference.body_types().iter().any(|t| t == b));

    body.insert("body_type".into(), json!(own_body.unwrap_or(body_type)));

    if own_body.is_none() {
        note(Level::Review, format!("The old profile has no body type, so {body_type} was used. It decides which scenes fit her: check that her model was built on that skeleton."));
    }

    for key in ["hitbox_height", "only_uses_double_beds"] {
        if let Some(value) = old.get(key) {
            body.insert(key.into(), value.clone());
        }
    }

    for key in ["gui_size", "gui_y_offset", "weapon_bone_rotation"] {
        if old.contains_key(key) {
            note(Level::Dropped, format!("\"{key}\" is not used any more (menu framing comes from her hitbox; the held item has held_item_offset, held_item_rotation and held_item_scale)."));
        }
    }

    let mut taste = Map::new();

    match old.get("tame_item").and_then(Value::as_str) {
        Some(item) => {
            taste.insert("tame_item".into(), json!(item));
        }
        None => note(Level::Review, "The old profile has no tame_item. Every girl needs one, so choose it in the girl's form.".into()),
    }

    let mut components = Map::new();

    components.insert("pleasurehorizons:identity".into(), Value::Object(identity));
    components.insert("pleasurehorizons:body".into(), Value::Object(body));
    components.insert("pleasurehorizons:taste".into(), Value::Object(taste));

    if let Some(attributes) = old.get("attributes").and_then(Value::as_object) {
        let stats: Map<String, Value> = ["health", "speed", "damage", "max_belly_inflation_size"].iter().filter_map(|k| attributes.get(*k).map(|v| (k.to_string(), v.clone()))).collect();

        if !stats.is_empty() {
            components.insert("pleasurehorizons:stats".into(), Value::Object(stats));
        }
    }

    let archetype = if old.get("type").and_then(Value::as_str) == Some("wild") { "wild" } else { "settler" };
    let girl = json!({ "id": id, "type": archetype, "components": components });

    note(Level::Kept, format!("{name} ({id}) as a {archetype}: her name, egg colours, hitbox, taste and stats."));

    let mut files = Vec::new();
    let mut scenes: Vec<ImportedScene> = Vec::new();
    let mut lang = BTreeMap::new();
    let mut taken: Vec<String> = existing_scenes.to_vec();

    files.push((format!("data/{namespace}/girls/{id}.json"), serde_json::to_string_pretty(&girl).unwrap() + "\n"));

    for scene in old.get("scenes").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
        let old_name = scene.get("name").and_then(Value::as_str).unwrap_or("scene").to_string();
        let kind = scene.get("scene_type").and_then(Value::as_str).map(str::to_lowercase).unwrap_or_else(|| if scene.get("is_bed_scene").and_then(Value::as_bool) == Some(true) { "on_bed".into() } else { "on_player".into() });
        let mut scene_id = slug(&old_name);

        if scene_id.is_empty() {
            scene_id = "scene".into();
        }

        if BUILTIN_SCENES.contains(&scene_id.as_str()) {
            note(Level::Changed, format!("The scene \"{old_name}\" would replace the game's own \"{scene_id}\", so it is \"{scene_id}_custom\" (it still shows as \"{old_name}\")."));
            scene_id = format!("{scene_id}_custom");
        }

        let base = scene_id.clone();
        let mut n = 2;

        while taken.contains(&scene_id) {
            scene_id = format!("{base}_{n}");
            n += 1;
        }

        taken.push(scene_id.clone());

        let mut settings = Map::new();

        settings.insert("type".into(), json!(kind));
        settings.insert("requires_affection".into(), json!(scene.get("required_level").and_then(Value::as_i64).unwrap_or(0).clamp(0, 10)));
        settings.insert("cum_threshold".into(), scene.get("cum_threshold").cloned().unwrap_or(json!(5)));

        for (from, to) in [("needs_to_strip", "needs_to_strip"), ("counts_towards_impregnation", "counts_towards_impregnation"), ("hide_player", "hide_player"), ("use_keyframe", "use_keyframe_events")] {
            settings.insert(to.into(), json!(scene.get(from).and_then(Value::as_bool).unwrap_or(false)));
        }

        settings.insert("bed_alignment_offset".into(), scene.get("bed_offset").cloned().unwrap_or(json!(0)));
        settings.insert("amount_of_loops".into(), scene.get("amount_of_loops").cloned().unwrap_or(json!(0)));
        settings.insert("body_types".into(), json!([own_body.unwrap_or(body_type)]));

        let tags: Vec<&str> = scene.get("tags").and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).filter(|t| reference.taste_tags.iter().any(|known| known == t)).collect()).unwrap_or_default();

        if !tags.is_empty() {
            settings.insert("tags".into(), json!(tags));
        }

        settings.insert("supported_girls".into(), json!([id]));

        files.push((format!("data/{namespace}/scenes/{scene_id}.json"), serde_json::to_string_pretty(&Value::Object(settings)).unwrap() + "\n"));
        lang.insert(format!("scene.pleasurehorizons.{scene_id}"), old_name.clone());
        scenes.push(ImportedScene { id: scene_id.clone(), old_name: old_name.clone(), roles: roles_of(scene, &kind) });
        note(Level::Kept, format!("The scene \"{old_name}\" is data/{namespace}/scenes/{scene_id}.json, for {id} only."));

        if ["intro_anim", "slow_anim", "fast_anim", "cum_anim", "lay_on_bed_anim", "bed_idle_anim", "anim"].iter().any(|k| scene.get(*k).is_some()) {
            note(Level::Review, format!("The animations of \"{old_name}\" are not imported yet: her animation file has to be converted and split into one file for each scene."));
        }
    }

    note(Level::Review, "Her model, texture, animations and sounds are not imported yet. Add them under data/pleasurehorizons/asset_sync/ (the checks say which are missing).".into());

    Ok(Imported { girl_id: id, scenes, files, lang, notes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{check_text, PackFacts};
    use crate::diagnostics::Severity;

    const OLD: &str = r##"{
        "format_version": 1, "id": "amber", "name": "Amber", "tame_item": "minecraft:golden_apple",
        "egg_color": { "primary": "#d9a066", "secondary": "#8f563b" },
        "hitbox_height": 1.8, "gui_size": 30, "gui_y_offset": 0.05, "weapon_bone_rotation": 100,
        "attributes": { "health": 20, "speed": 0.2, "damage": 2, "max_belly_inflation_size": 550 },
        "scenes": [
          { "name": "Doggy", "scene_type": "ON_BED", "required_level": 4, "use_keyframe": true, "bed_offset": 0.5, "intro_anim": "a", "tags": ["kind", "nonsense"] },
          { "name": "Hug Time", "required_level": 99 }
        ]
    }"##;

    #[test]
    fn brings_in_a_girl_and_her_scenes() {
        let reference = Reference::builtin();
        let imported = import_profile(OLD, "amber", "jenny", "pleasurehorizons", &[], &reference).unwrap();
        let names: Vec<&str> = imported.files.iter().map(|(p, _)| p.as_str()).collect();

        assert_eq!(names, vec!["data/pleasurehorizons/girls/amber.json", "data/pleasurehorizons/scenes/doggy_custom.json", "data/pleasurehorizons/scenes/hug_time.json"]);
        assert_eq!(imported.lang["scene.pleasurehorizons.doggy_custom"], "Doggy");

        for (path, text) in &imported.files {
            let errors: Vec<_> = check_text(path, text, &reference, &PackFacts::default()).into_iter().filter(|d| d.severity == Severity::Error).collect();

            assert!(errors.is_empty(), "{path}: {:?}", errors.iter().map(|e| &e.message).collect::<Vec<_>>());
        }

        assert!(imported.files[1].1.contains("\"requires_affection\": 4"));
        assert!(imported.files[1].1.contains("\"kind\"") && !imported.files[1].1.contains("nonsense"));
        assert!(imported.files[2].1.contains("\"requires_affection\": 10"), "affection is kept within what the game allows");
        assert!(imported.notes.iter().any(|n| n.level == Level::Dropped && n.message.contains("gui_size")));
        assert!(imported.notes.iter().any(|n| n.level == Level::Changed && n.message.contains("doggy_custom")));
    }

    #[test]
    fn refuses_what_it_cannot_use() {
        let reference = Reference::builtin();

        assert!(import_profile("{ not json", "x", "jenny", "p", &[], &reference).is_err());
        assert!(import_profile(r#"{ "id": "a", "components": {} }"#, "x", "jenny", "p", &[], &reference).unwrap_err().to_string().contains("already"));
        assert!(import_profile(r#"{ "id": "a" }"#, "x", "nope", "p", &[], &reference).is_err());
    }

    #[test]
    fn avoids_scene_ids_the_pack_has() {
        let reference = Reference::builtin();
        let imported = import_profile(r#"{ "id": "a", "tame_item": "minecraft:allium", "scenes": [ { "name": "Hug" }, { "name": "hug" } ] }"#, "a", "jenny", "p", &["hug".to_string()], &reference).unwrap();
        let names: Vec<&str> = imported.files.iter().map(|(p, _)| p.as_str()).collect();

        assert_eq!(names, vec!["data/p/girls/a.json", "data/p/scenes/hug_2.json", "data/p/scenes/hug_3.json"]);
    }
}
