//! Resource packs made for the mod this one grew out of (Fapcraft, whose assets live in `assets/sexmod/`). Such a pack has one model, one animation file holding every clip, her sounds
//! in a folder for each cue and a language file for the whole mod, with no profile and no cue table. Rather than teach the importer a second layout, the pack is rewritten into the
//! layout it already understands (a nude and a dressed model, `animations/<id>.animation.json`, a cue table, a profile) in a temporary folder, and the import runs on that.

use crate::check::walk;
use crate::error::{Error, Result};
use crate::import::{Level, Note};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// What was found in a pack of the old mod's layout, written out in the layout the importer reads.
pub struct Rewritten {
    pub girls: Vec<String>,
    /// Each girl's profile (the text of an old profile file) made from what the pack says: her name and the scenes her clips make up.
    pub profiles: BTreeMap<String, String>,
    pub notes: Vec<Note>,
}

/// Whether the folder of a pack's assets is laid out for the old mod: a model not in `geo/nude` or `geo/dressed`.
pub fn is_layout(root: &Path) -> bool {
    let has = |dir: &str| fs::read_dir(root.join("geo").join(dir)).map(|mut entries| entries.any(|e| e.is_ok_and(|e| e.file_name().to_string_lossy().ends_with(".geo.json")))).unwrap_or(false);

    !has("nude") && !has("dressed") && !models_of(root).is_empty()
}

/// The models of the pack: `geo/<name>/<name>.geo.json` or `geo/<name>.geo.json`, by the name they carry.
fn models_of(root: &Path) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();

    for rel in walk(&root.join("geo")) {
        let Some(file) = rel.rsplit('/').next() else { continue };
        let Some(stem) = file.strip_suffix(".geo.json") else { continue };

        if !found.iter().any(|(name, _)| name == stem) {
            found.push((stem.to_string(), root.join("geo").join(&rel)));
        }
    }

    found
}

fn id_of(text: &str) -> String {
    let mapped: String = text.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { '_' }).collect();

    mapped.split('_').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("_")
}

/// Words from a clip's name: `touch_boobs` is "Touch boobs".
fn title_of(stem: &str) -> String {
    let mut text = stem.replace('_', " ");

    if let Some(first) = text.get(0..1).map(str::to_uppercase) {
        text.replace_range(0..1, &first);
    }

    text
}

/// The part a clip plays in a scene, from the end of its name: `touch_boobs_slow1` is the `slow` of `touch_boobs`.
fn role_of(name: &str) -> Option<(&str, &'static str)> {
    for role in ["intro", "slow", "fast", "cum"] {
        let Some(at) = name.rfind(&format!("_{role}")) else { continue };
        let tail = &name[at + 1 + role.len()..];

        if at > 0 && tail.chars().all(|c| c.is_ascii_digit()) {
            return Some((&name[..at], role));
        }
    }

    None
}

/// The scenes her clips make up: a name that has a `_slow` or a `_fast` (and any `_intro`, `_cum`) is one scene, and the clips with another ending of the same name are the
/// variants of that part.
fn scenes_of(clips: &[String]) -> Vec<Value> {
    let mut grouped: BTreeMap<String, BTreeMap<&'static str, Vec<String>>> = BTreeMap::new();

    for clip in clips {
        if let Some((stem, role)) = role_of(clip) {
            grouped.entry(stem.to_string()).or_default().entry(role).or_default().push(clip.clone());
        }
    }

    let mut scenes = Vec::new();

    for (stem, parts) in grouped {
        if !parts.contains_key("slow") && !parts.contains_key("fast") {
            continue;
        }

        let mut scene = Map::new();

        scene.insert("name".into(), json!(title_of(&stem)));
        scene.insert("use_keyframe".into(), json!(true));

        for (role, field) in [("intro", "intro_anim"), ("slow", "slow_anim"), ("fast", "fast_anim"), ("cum", "cum_anim")] {
            if let Some(names) = parts.get(role) {
                scene.insert(field.into(), if names.len() == 1 { json!(names[0]) } else { json!(names) });
            }
        }

        scenes.push(Value::Object(scene));
    }

    scenes
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    fs::write(path, bytes).map_err(|e| Error::io(path, e))
}

/// A name for a file or a folder that asset sync accepts.
fn file_name_of(text: &str) -> String {
    let mapped: String = text.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { c } else { '_' }).collect();

    mapped
}

/// Rewrites the pack at `root` (the folder of its assets) into the layout the importer reads, inside `into`.
pub fn rewrite(root: &Path, into: &Path) -> Result<Rewritten> {
    let models = models_of(root);
    let mut notes = Vec::new();

    /* One model is one girl, named for whom the pack's sounds and language file are for; several models are several girls, named for the models. */
    let sound_owners: BTreeSet<String> = fs::read_dir(root.join("sounds").join("girls")).map(|entries| entries.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
    let single = models.len() == 1;
    let mut lang_names: BTreeMap<String, String> = BTreeMap::new();

    if let Ok(text) = fs::read_to_string(root.join("lang").join("en_us.lang")) {
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            if let Some((key, value)) = line.split_once('=') {
                if let Some(who) = key.trim().strip_prefix("entity.").and_then(|k| k.strip_suffix(".name")) {
                    lang_names.insert(who.to_lowercase(), value.trim().to_string());
                }
            }
        }
    }

    let mut girls = Vec::new();
    let mut profiles = BTreeMap::new();

    for (stem, geo) in &models {
        let owner = if single && sound_owners.len() == 1 { sound_owners.iter().next().cloned() } else if single { sound_owners.iter().find(|o| lang_names.contains_key(&o.to_lowercase())).cloned() } else { None };
        let id = id_of(&owner.clone().unwrap_or_else(|| stem.clone()));

        if id.is_empty() {
            continue;
        }

        let name = owner.as_ref().and_then(|o| lang_names.get(&o.to_lowercase())).cloned().unwrap_or_else(|| title_of(&id));

        /* The model, for both of her models: the old mod's girl was one model whose clothes were bones of it. */
        let text = fs::read(geo).map_err(|e| Error::io(geo, e))?;

        write(&into.join("geo").join("nude").join(format!("{id}.geo.json")), &text)?;
        write(&into.join("geo").join("dressed").join(format!("{id}.geo.json")), &text)?;

        /* Her animations, named for her. */
        let mut clips: Vec<String> = Vec::new();
        let animation = walk(&root.join("animations")).into_iter().find(|rel| rel.rsplit('/').next().is_some_and(|f| f.strip_suffix(".animation.json").is_some_and(|s| s == stem)));

        if let Some(rel) = animation {
            let path = root.join("animations").join(&rel);
            let mut doc: Value = serde_json::from_str(&fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?).map_err(|e| Error::invalid(format!("{rel} is not valid JSON: {e}")))?;

            if let Some(clip_map) = doc.get_mut("animations").and_then(Value::as_object_mut) {
                let renamed: Map<String, Value> = clip_map
                    .iter()
                    .map(|(key, value)| {
                        let short = key.strip_prefix("animation.").and_then(|rest| rest.split_once('.')).map_or(key.as_str(), |(_, name)| name);

                        clips.push(short.to_string());
                        (format!("animation.{id}.{short}"), value.clone())
                    })
                    .collect();

                *clip_map = renamed;
            }

            write(&into.join("animations").join(format!("{id}.animation.json")), serde_json::to_string(&doc).unwrap().as_bytes())?;
        } else {
            notes.push(Note { level: Level::Error, message: format!("The pack has no animations/**/{stem}.animation.json for {id}.") });
        }

        /* Her texture: the one named for the model is hers, any other becomes an extra. */
        let textures = walk(&root.join("textures")).into_iter().filter(|rel| rel.to_lowercase().ends_with(".png")).collect::<Vec<_>>();
        let main = textures.iter().find(|rel| rel.rsplit('/').next().is_some_and(|f| f.eq_ignore_ascii_case(&format!("{stem}.png")))).or_else(|| textures.first());

        for rel in &textures {
            let file = rel.rsplit('/').next().unwrap_or(rel).trim_end_matches(".png").trim_end_matches(".PNG");
            let target = if Some(rel) == main { format!("{id}.png") } else { format!("{id}_{}.png", file_name_of(file)) };

            write(&into.join("textures").join("entities").join(target), &fs::read(root.join("textures").join(rel)).map_err(|e| Error::io(rel, e))?)?;
        }

        if textures.len() > 1 {
            notes.push(Note { level: Level::Review, message: format!("The pack has {} textures; \"{}\" is her skin and the others are kept as {id}_<name>.png for you to use or delete.", textures.len(), main.map_or("", |m| m.rsplit('/').next().unwrap_or(m))) });
        }

        /* Her sounds: a folder for each cue becomes one sound event, and the cue (an effect name in her animations) plays it. */
        let mut events: Map<String, Value> = Map::new();
        let mut cue_table: Vec<Value> = Vec::new();
        let mut sound_files = 0;

        for rel in walk(&root.join("sounds")).into_iter().filter(|r| r.to_lowercase().ends_with(".ogg")) {
            let parts: Vec<&str> = rel.split('/').collect();
            let inside: &[&str] = if parts.len() > 2 && parts[0] == "girls" { &parts[2..] } else { &parts };
            let file = inside.last().copied().unwrap_or("").trim_end_matches(".ogg").trim_end_matches(".OGG");
            let cue = if inside.len() > 1 { inside[inside.len() - 2].to_lowercase() } else { file.trim_end_matches(|c: char| c.is_ascii_digit()).to_lowercase() };

            if cue.is_empty() {
                continue;
            }

            let cue_id = file_name_of(&cue);
            let target = format!("{id}/{cue_id}/{}", file_name_of(file));

            write(&into.join("sounds").join(format!("{target}.ogg")), &fs::read(root.join("sounds").join(&rel)).map_err(|e| Error::io(&rel, e))?)?;
            events.entry(format!("{id}.{cue_id}")).or_insert_with(|| json!({ "sounds": [] }))["sounds"].as_array_mut().unwrap().push(json!(format!("pleasurehorizons:{target}")));
            sound_files += 1;
        }

        for key in events.keys() {
            let cue = key.trim_start_matches(&format!("{id}."));

            cue_table.push(json!({ "key": cue, "sounds": [format!("pleasurehorizons:{key}")] }));
        }

        if sound_files > 0 {
            write(&into.join("sounds.json"), serde_json::to_string_pretty(&events).unwrap().as_bytes())?;
            write(&into.join("keyframe_events").join(format!("{id}.json")), json!({ "girl_id": id, "events": cue_table }).to_string().as_bytes())?;
            notes.push(Note { level: Level::Kept, message: format!("{sound_files} sound(s) in {} cue folder(s). A cue in her animations plays the sound of the same name.", events.len()) });
        }

        /* Her profile, made from what is known. */
        let scenes = scenes_of(&clips);

        if !scenes.is_empty() {
            notes.push(Note { level: Level::Review, message: format!("{} scene(s) were found from her clip names ({}). Their type, affection and tags are guesses: open each scene file and set them.", scenes.len(), scenes.iter().filter_map(|s| s["name"].as_str()).collect::<Vec<_>>().join(", ")) });
        }

        profiles.insert(id.clone(), json!({ "id": id, "name": name, "tame_item": "minecraft:golden_apple", "hitbox_height": 1.8, "attributes": { "health": 20.0, "speed": 0.2, "damage": 2.0 }, "scenes": scenes }).to_string());
        girls.push(id);
    }

    notes.push(Note { level: Level::Changed, message: "This pack is in the old mod's layout, with one model. It is used for both her nude and her dressed model: the clothes are bones of it, so make a dressed version in Blockbench if you want her to wear them.".into() });

    if root.join("lang").is_dir() {
        notes.push(Note { level: Level::Dropped, message: "The pack's language files describe the whole old mod, so only her name was kept from them.".into() });
    }

    Ok(Rewritten { girls, profiles, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_scenes_from_the_names_of_clips() {
        let clips: Vec<String> = ["idle", "walk", "touch_boobs_intro", "touch_boobs_slow", "touch_boobs_slow1", "touch_boobs_fast", "touch_boobs_cum", "sitting_slow", "sitting_fast", "fast_walk", "head_pat"].iter().map(|s| s.to_string()).collect();
        let scenes = scenes_of(&clips);

        assert_eq!(scenes.len(), 2);

        let touch = scenes.iter().find(|s| s["name"] == "Sitting").unwrap();

        assert_eq!(touch["slow_anim"], "sitting_slow");
        assert!(scenes.iter().any(|s| s["name"] == "Touch boobs" && s["slow_anim"] == json!(["touch_boobs_slow", "touch_boobs_slow1"]) && s["intro_anim"] == "touch_boobs_intro"));
    }

    #[test]
    fn a_clip_that_only_ends_like_a_part_is_not_a_scene() {
        assert!(scenes_of(&["fast_walk".into(), "slowdown".into(), "intro".into()]).is_empty());
        assert_eq!(role_of("sitting_cum"), Some(("sitting", "cum")));
        assert_eq!(role_of("cum"), None);
    }
}
