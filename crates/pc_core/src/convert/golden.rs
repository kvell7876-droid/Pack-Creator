//! The converter is a port, so it is held to the original: the game's own sources (`asset_sources/`) are converted and every file is compared, byte for byte, with the
//! one the game ships (made by the TypeScript converter).

use super::animation::{self, Options};
use super::events::{self, Mapping};
use super::model;
use crate::check::walk;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// FNV-1a, 64 bit: a hash that is the same on every machine and version.
fn fnv(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;

    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}

/// Files the game ships changed after they were converted (physics added to four models, the base animation files edited), so they cannot be compared with the shipped
/// ones. The original TypeScript converter was run on the same sources, and these are the hashes of what it wrote: ours must be the same.
const ORIGINAL_CONVERTER: [(&str, &str); 6] = [
    ("animations/base.strutanim", "ed1564c15cfd7fc6"),
    ("animations/base.mika.strutanim", "5255e81aad7719ba"),
    ("models/dressed/lucy.strutmodel", "3764c523a25d2013"),
    ("models/dressed/mika.strutmodel", "be458d36d82736ea"),
    ("models/nude/lucy.strutmodel", "f74afce9cea9b955"),
    ("models/nude/mika.strutmodel", "61f8144c9c51e15d"),
];

/// Models that were rebuilt in Blockbench and saved as `.strutmodel` (Strut's toolkit writes its own file, with free-form meshes, a skinned armature and the editor's bone colours,
/// none of which Bedrock JSON can hold). The binary file is their master, so converting their `asset_sources` is not expected to give it back and they are left out of the comparison.
/// Their old `geo.json` files stay because the shipped animations are stamped with that skeleton's fingerprint; see `asset_sources/README.md`.
const EDITED_AFTER_CONVERSION: [&str; 4] = [
    "models/dressed/momo.strutmodel",
    "models/nude/momo.strutmodel",
    "models/dressed/slime.strutmodel",
    "models/nude/slime.strutmodel",
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn first_difference(a: &[u8], b: &[u8]) -> String {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(at) => format!("first differs at byte {at} (ours {:#04x}, shipped {:#04x}); lengths {} and {}", a[at], b[at], a.len(), b.len()),
        None => format!("same up to the shorter one; lengths {} (ours) and {} (shipped)", a.len(), b.len()),
    }
}

/// The id a shared animation file is named for: `base.lucy.animation.json` and `doggy-momo.scene.json` are for `lucy` and `momo`.
fn model_id_of(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".animation.json").map(|s| s.rsplit_once('.').map(|(_, id)| id)).or_else(|| name.strip_suffix(".scene.json").map(|s| s.rsplit_once('-').map(|(_, id)| id)));

    stem.flatten().filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).map(String::from)
}

#[test]
fn the_games_own_sources_convert_to_the_files_it_ships() {
    let sources = repo().join("asset_sources");
    let shipped = repo().join("src/main/resources/assets/pleasurehorizons");

    if !sources.exists() || !shipped.exists() {
        return;
    }

    let mut mismatches: Vec<String> = Vec::new();
    let mut checked = 0;

    /* Models first: an animation is stamped with the skeleton of the model it is named for (the nude one names it). */
    let mut fingerprints: HashMap<String, i32> = HashMap::new();

    for rel in walk(&sources.join("geo")) {
        let Some(stem) = rel.strip_suffix(".geo.json") else { continue };
        let text = std::fs::read_to_string(sources.join("geo").join(&rel)).unwrap();
        let converted = model::convert_geo(&serde_json::from_str(&text).unwrap(), Some(&rel));
        let bytes = model::encode(&converted.model).unwrap();
        let id = stem.rsplit('/').next().unwrap().to_string();

        if !fingerprints.contains_key(&id) || rel.contains("nude/") {
            fingerprints.insert(id, model::fingerprint(&converted.model));
        }

        let expected = std::fs::read(shipped.join("strut/models").join(format!("{stem}.strutmodel")));

        let key = format!("models/{stem}.strutmodel");

        if EDITED_AFTER_CONVERSION.contains(&key.as_str()) {
            continue;
        }

        if let Some((_, hash)) = ORIGINAL_CONVERTER.iter().find(|(name, _)| *name == key) {
            if fnv(&bytes) == *hash { checked += 1 } else { mismatches.push(format!("model {rel}: not what the original converter wrote ({})", fnv(&bytes))) }

            continue;
        }

        match expected {
            Ok(expected) if expected == bytes => checked += 1,
            Ok(expected) => mismatches.push(format!("model {rel}: {}", first_difference(&bytes, &expected))),
            Err(_) => mismatches.push(format!("model {rel}: no shipped file")),
        }
    }

    let rules = Mapping::new();
    let mut tokens: BTreeMap<String, usize> = BTreeMap::new();
    let mut cues: BTreeMap<String, usize> = BTreeMap::new();

    for rel in walk(&sources.join("animations")) {
        let Some(stem) = rel.strip_suffix(".animation.json").or_else(|| rel.strip_suffix(".scene.json")) else { continue };
        let text = std::fs::read_to_string(sources.join("animations").join(&rel)).unwrap();
        let name = rel.rsplit('/').next().unwrap();
        let fingerprint = model_id_of(name).and_then(|id| fingerprints.get(&id).copied()).unwrap_or(0);
        let mut mapper = events::marker_mapper(&rules, &mut tokens, &mut cues);
        let converted = animation::convert(&serde_json::from_str(&text).unwrap(), Options { file: Some(rel.clone()), fingerprint, map_event: Some(&mut mapper) });
        let bytes = animation::encode(&converted.animation).unwrap();

        let key = format!("animations/{stem}.strutanim");

        if let Some((_, hash)) = ORIGINAL_CONVERTER.iter().find(|(name, _)| *name == key) {
            if fnv(&bytes) == *hash { checked += 1 } else { mismatches.push(format!("animation {rel}: not what the original converter wrote ({})", fnv(&bytes))) }

            continue;
        }

        match std::fs::read(shipped.join("strut/animations").join(format!("{stem}.strutanim"))) {
            Ok(expected) if expected == bytes => checked += 1,
            Ok(expected) => mismatches.push(format!("animation {rel}: {}", first_difference(&bytes, &expected))),
            Err(_) => mismatches.push(format!("animation {rel}: no shipped file")),
        }
    }

    /* The old registries are kept in the sources: the game no longer reads them, but they are what the bindings were made from. */
    for entry in std::fs::read_dir(sources.join("keyframe_events")).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        if !name.ends_with(".json") || name == "example.json" {
            continue;
        }

        let registry = events::parse_keyframe_events(&serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap(), &name);
        let set = events::binding_set_for(&registry, tokens.keys().cloned());
        let ours = serde_json::to_string_pretty(&set).unwrap() + "\n";

        match std::fs::read_to_string(shipped.join("strut/event_bindings/girls").join(format!("{}.json", registry.girl_id))) {
            /* The game's files have one more binding, the fluid of the `cumin` cue, added by hand after the conversion. */
            Ok(expected) => {
                let mut value: serde_json::Value = serde_json::from_str(&expected).unwrap();

                value["bindings"].as_object_mut().unwrap().shift_remove("cumin");

                let theirs = serde_json::to_string_pretty(&value).unwrap() + "\n";

                if theirs == ours { checked += 1 } else { mismatches.push(format!("bindings {name}: {}", first_difference(ours.as_bytes(), theirs.as_bytes()))) }
            }
            Err(_) => mismatches.push(format!("bindings {name}: no shipped file")),
        }
    }

    println!("matched {checked} files");

    for line in &mismatches {
        println!("MISMATCH {line}");
    }

    assert!(mismatches.is_empty(), "{} files differ from the ones the game ships:\n{}", mismatches.len(), mismatches.join("\n"));
}
