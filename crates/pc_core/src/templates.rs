//! The text of a new file. What the "new girl" or "new scene" dialog makes is a valid file the user then fills in.

use crate::error::{Error, Result};
use crate::layout::{folder_for, FileKind};
use crate::project::ProjectMeta;
use serde_json::{json, Value};

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap() + "\n"
}

/// What a new girl needs to be a girl: her kind, her id, her name, her body type and the item that tames her.
pub struct GirlSpec<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub archetype: &'a str,
    pub body_type: &'a str,
    pub tame_item: &'a str,
    pub hitbox_height: f64,
}

pub fn girl(spec: &GirlSpec) -> String {
    pretty(&json!({
        "id": spec.id,
        "type": spec.archetype,
        "components": {
            "pleasurehorizons:identity": { "name": spec.name },
            "pleasurehorizons:body": { "body_type": spec.body_type, "hitbox_height": spec.hitbox_height },
            "pleasurehorizons:taste": { "tame_item": spec.tame_item }
        }
    }))
}

pub fn scene(scene_type: &str) -> String {
    pretty(&json!({
        "type": scene_type,
        "requires_affection": 0,
        "body_types": [],
        "tags": []
    }))
}

pub fn trait_file(category: &str) -> String {
    pretty(&json!({
        "category": category,
        "description": "What she does, in a sentence.",
        "params": {},
        "goals": []
    }))
}

pub fn dialogue(topic: &str) -> String {
    pretty(&json!([
        {
            "id": "first_line",
            "topic": topic,
            "tags": [],
            "player_line": "What the player says.",
            "reply_good": "How she answers when it goes well.",
            "reply_bad": "How she answers when it does not."
        }
    ]))
}

/// A small talk file with one line of each kind: what two settlers say, and what one says when she walks over to the player.
pub fn social(kind: &str) -> String {
    if kind == "offer" {
        return pretty(&json!([
            { "id": "first_offer", "kind": "offer", "text": "Have you got a minute? Come and talk to me." }
        ]));
    }

    pretty(&json!([
        { "id": "first_line", "kind": "line", "text": "How is your day going, {partner}?", "outcome": ["easy", "warm"] }
    ]))
}

/// A file id: lower case letters, digits and underscores.
pub fn id_problem(id: &str) -> Option<String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        Some("Use lower case letters, digits and underscores only, such as \"amber\".".into())
    } else {
        None
    }
}

/// The path of a new file of `kind` called `id`, in the pack's own namespace.
pub fn path_for(meta: &ProjectMeta, kind: &FileKind, id: &str) -> Result<String> {
    if let Some(problem) = id_problem(id) {
        return Err(Error::invalid(problem));
    }

    let folder = folder_for(kind).ok_or_else(|| Error::invalid("That kind of file has no folder of its own."))?;

    Ok(format!("data/{}/{folder}/{id}.json", meta.namespace))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{check_text, PackFacts};
    use crate::diagnostics::Severity;
    use crate::reference::Reference;

    fn errors(rel: &str, text: &str) -> Vec<String> {
        check_text(rel, text, &Reference::builtin(), &PackFacts::default()).into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
    }

    #[test]
    fn what_the_tool_makes_passes_its_own_checks() {
        let girl = girl(&GirlSpec { id: "amber", name: "Amber", archetype: "settler", body_type: "jenny", tame_item: "minecraft:allium", hitbox_height: 1.8 });

        assert!(errors("data/x/girls/amber.json", &girl).is_empty());
        assert!(errors("data/x/scenes/s.json", &scene("on_bed")).is_empty());
        assert!(errors("data/x/girl_traits/t.json", &trait_file("movement")).is_empty());
        assert!(errors("data/x/dialogue/d.json", &dialogue("chat")).is_empty());
        assert!(errors("data/x/social/s.json", &social("line")).is_empty());
        assert!(errors("data/x/social/s.json", &social("offer")).is_empty());
    }

    #[test]
    fn ids_and_paths() {
        let meta = ProjectMeta { format: 1, name: "n".into(), namespace: "mypack".into(), author: String::new(), description: String::new(), contents: vec![], created: 0 };

        assert_eq!(path_for(&meta, &FileKind::Girl, "amber").unwrap(), "data/mypack/girls/amber.json");
        assert!(path_for(&meta, &FileKind::Girl, "Amber").is_err());
        assert!(path_for(&meta, &FileKind::Model, "a").is_err());
    }
}
