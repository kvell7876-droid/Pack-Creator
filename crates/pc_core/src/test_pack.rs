//! Builds the test pack: every girl of the old resource packs in the game's `run/resourcepacks` folder, imported the way the Import button does it (her old profile from
//! `run/config/pleasurehorizons/girl` when she has one), so there are real models, scenes, animations, cues and physics to try the viewer, the scene simulator and the physics strip on.
//!
//! The girls are other people's work, so the folder it makes (`test_pack/` next to the program) is not kept in git. Make it with
//! `cargo test -p pc_core build_test_pack -- --ignored --nocapture`.

use crate::check::{check_project, walk};
use crate::convert::events::Mapping;
use crate::diagnostics::Severity;
use crate::oldpack::{import_girl, open, Request};
use crate::reference::Reference;
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const NS: &str = "testpack";

/// The game's own girls and the body each is built on: a girl whose bones are most like one of these is given that body.
const BODIES: [(&str, &str); 5] = [("lucy", "jenny"), ("mika", "ellie"), ("momo", "bia"), ("bee_girl", "goblin"), ("kobold", "kobold")];

fn put(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);

    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// What the game ships for the body types, as bone names.
fn shipped_bones() -> Vec<(&'static str, BTreeSet<String>)> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../src/main/resources/assets/pleasurehorizons/strut/models/nude");

    BODIES
        .iter()
        .filter_map(|(girl, body)| {
            let bytes = fs::read(assets.join(format!("{girl}.strutmodel"))).ok()?;
            let model = crate::strut_model::read(&bytes).ok()?;

            Some((*body, model.group_bones.iter().map(|b| b.name.clone()).collect()))
        })
        .collect()
}

/// The body whose skeleton her nude model is most like, or `jenny` when none is close.
fn body_for(pack: &Path, id: &str, shipped: &[(&'static str, BTreeSet<String>)]) -> &'static str {
    let Ok(text) = fs::read_to_string(pack.join("geo/nude").join(format!("{id}.geo.json"))) else { return "jenny" };
    let Ok(json) = serde_json::from_str::<Value>(&text) else { return "jenny" };
    let mut names: BTreeSet<String> = BTreeSet::new();

    for geometry in json.get("minecraft:geometry").and_then(Value::as_array).into_iter().flatten() {
        for bone in geometry.get("bones").and_then(Value::as_array).into_iter().flatten() {
            if let Some(name) = bone.get("name").and_then(Value::as_str) {
                names.insert(name.to_string());
            }
        }
    }

    let mut best = ("jenny", 0.0);

    for (body, bones) in shipped {
        let shared = bones.intersection(&names).count() as f64;
        let score = shared / (bones.len() + names.len()) as f64 * 2.0;

        if score > best.1 {
            best = (body, score);
        }
    }

    best.0
}

fn merge(root: &Path, rel: &str, entries: &std::collections::BTreeMap<String, Value>, replace: bool) {
    let path = root.join(rel);
    let mut object: Map<String, Value> = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();

    for (key, value) in entries {
        if replace || !object.contains_key(key) {
            object.insert(key.clone(), value.clone());
        }
    }

    put(root, rel, (serde_json::to_string_pretty(&object).unwrap() + "\n").as_bytes());
}

#[test]
#[ignore = "builds test_pack from the old resource packs in the game's run folder"]
fn build_test_pack() {
    let run: PathBuf = std::env::var("PH_RUN").map(PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../run"));
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_pack");
    let reference = Reference::builtin();
    let shipped = shipped_bones();

    let _ = fs::remove_dir_all(&out);
    put(&out, "pack.mcmeta", (serde_json::to_string_pretty(&json!({ "pack": { "pack_format": crate::check::PACK_FORMAT, "description": "Every girl of the old resource packs, imported" } })).unwrap() + "\n").as_bytes());
    put(&out, ".packcreator/project.json", (serde_json::to_string_pretty(&json!({ "format": 1, "name": "Test Pack", "namespace": NS, "author": "", "description": "The old resource packs' girls, to try the tool on", "contents": ["girls", "scenes"], "created": 0 })).unwrap() + "\n").as_bytes());

    let mut zips: Vec<PathBuf> = fs::read_dir(run.join("resourcepacks")).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "zip") && !p.file_name().unwrap().to_string_lossy().starts_with("CustomModels")).collect();

    zips.sort();

    let mut made: BTreeSet<String> = BTreeSet::new();

    for zip in zips {
        let Ok(pack) = open(&zip) else {
            println!("skipped {}: not an old girl's pack", zip.display());
            continue;
        };

        for (position, girl) in pack.girls.clone().iter().enumerate() {
            let id = girl.to_lowercase();

            if made.contains(&id) {
                println!("skipped {girl} from {}: a girl with that id is already in", zip.file_name().unwrap().to_string_lossy());
                continue;
            }

            let profile = fs::read_to_string(run.join("config/pleasurehorizons/girl").join(format!("{id}.json"))).ok();
            let body = body_for(&pack.root, girl, &shipped);
            let existing: HashSet<String> = walk(&out).into_iter().collect();
            let scenes: Vec<String> = existing.iter().filter_map(|rel| rel.strip_prefix(&format!("data/{NS}/scenes/")).and_then(|r| r.strip_suffix(".json")).map(String::from)).collect();
            let taken = crate::oldpack::taken_tame_items(&out);
            let result = import_girl(
                &pack,
                &Request { raw_id: girl, profile: profile.as_deref(), body_type: body, namespace: NS, liquid: true, existing_files: &existing, existing_scenes: &scenes, rules: &Mapping::new(), physics_preset: true, skip_shared: position > 0, taken_tame_items: &taken, scene_plan: None },
                &reference,
            );

            match result {
                Ok(outcome) => {
                    for (rel, bytes) in &outcome.files {
                        put(&out, rel, bytes);
                    }

                    for (rel, entries) in &outcome.merges {
                        merge(&out, rel, entries, rel.contains("/lang/"));
                    }

                    made.insert(id.clone());
                    println!("{id}: {} files, body {body}{}", outcome.files.len(), if profile.is_some() { "" } else { ", no old profile" });
                }
                Err(error) => println!("skipped {girl}: {error}"),
            }
        }
    }

    println!("{} girls imported", made.len());

    let problems = check_project(&out, &reference);
    let errors = problems.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = problems.iter().filter(|d| d.severity == Severity::Warning).count();

    println!("the checks find {errors} error(s) and {warnings} warning(s)");

    for d in problems.iter().filter(|d| d.severity == Severity::Error).take(15) {
        println!("  {} {}: {}", d.file, d.code, d.message);
    }
}
