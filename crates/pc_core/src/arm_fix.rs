//! Fixes the lower arms of a model that were built with a rotated bone.
//!
//! Some models have a lower arm bone with a rotation on X (35, 140, -167.5 ...) and compensate for it by rotating the cubes in the bone the other way (-35, -140 ...), around a
//! pivot of their own. It looks right at rest, but the arm bone is rotated, and anything that hangs off it (the hand item) is rotated with it, which the game's hand item
//! renderer does not expect. This sets the bone's rotation to zero and moves the rotation into the cubes, so the arm looks exactly as it did.
//!
//! It is done the way Blockbench does it when a cube's pivot is moved and the cube must stay where it is. For each cube:
//!
//! 1. The cube's pivot is moved to the bone's pivot. A cube rotated by `c` about its pivot `O` is where it is because of that pivot, so moving the pivot by `d = P - O` moves the
//!    box by `d - R(c)^-1 d`, and the box and the rotation are then the same in the world as before. (This is why the box's `from` and `to` change and are not simply left alone.)
//! 2. The cube and the bone now turn about the same point, so their rotations add: the cube takes `bone + cube` and the bone takes zero.
//!
//! It also removes the old item bones that hang off the arms: `weapon`, `offhand` / `offHand` and the `customHand` ones (`customHandL`, `customHandR`). The game's item renderer places
//! held items itself now, so these are only in the way. A bone is only removed when nothing hangs off it and it holds no cubes, and no armature is attached to it; the bones after it
//! move up one place, and the parents that pointed past it are renumbered, so the model is otherwise the same.
//!
//! Only X rotations are handled, which is what these arms have; a bone or cube that is rotated on another axis is left alone and reported, and so is a bone with a mesh in it,
//! because moving the rotation into a cube without moving it into the mesh would leave them apart. The bytes of the file are changed in place (a few floats), so everything else
//! in it (skinning, extensions, the order of things) stays exactly as it was.

use crate::strut_model::{self, Model};
use serde::Serialize;

/// A bone that was (or would be) fixed.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fixed {
    pub bone: String,
    /// The bone's rotation on X before it was set to zero, in degrees.
    pub old_rotation: f32,
    /// The cubes that took the rotation, and what each one's rotation was and now is.
    pub cubes: Vec<CubeChange>,
    /// Bones that hang off this one (the hand), which keep their own numbers: they are no longer turned by this bone, so they sit where their own pivot puts them.
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CubeChange {
    pub name: String,
    pub old_rotation: f32,
    pub new_rotation: f32,
}

/// A lower arm bone that has a rotation but was left alone, and why.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Skipped {
    pub bone: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Report {
    pub fixed: Vec<Fixed>,
    pub skipped: Vec<Skipped>,
    /// The old item bones that were (or would be) taken out.
    pub removed: Vec<String>,
}

const EPSILON: f32 = 1e-3;

fn is_lower_arm(name: &str) -> bool {
    name.to_lowercase().contains("lowerarm")
}

/// The box offset that keeps a cube where it is when its pivot moves by `d` (see the top of the file). `c` is the cube's rotation on X, in degrees.
pub fn pivot_shift(d: [f64; 3], c: f64) -> [f64; 3] {
    let (sin, cos) = c.to_radians().sin_cos();

    [0.0, d[1] - (d[1] * cos + d[2] * sin), d[2] - (-d[1] * sin + d[2] * cos)]
}

/// Brings an angle in degrees back into -180 to 180.
fn wrap(degrees: f64) -> f64 {
    let mut value = degrees % 360.0;

    if value > 180.0 {
        value -= 360.0;
    } else if value < -180.0 {
        value += 360.0;
    }

    value
}

/// What would change in this model, and the new bytes. The file is only changed when something was fixed.
pub fn fix(bytes: &[u8]) -> Result<(Vec<u8>, Report), String> {
    let model = strut_model::read(bytes)?;
    let mut out = bytes.to_vec();
    let mut report = Report::default();

    for (index, bone) in model.group_bones.iter().enumerate() {
        if !is_lower_arm(&bone.name) || bone.rotation[0].abs() <= EPSILON && bone.rotation[1].abs() <= EPSILON && bone.rotation[2].abs() <= EPSILON {
            continue;
        }

        if let Some(reason) = refusal(&model, index) {
            report.skipped.push(Skipped { bone: bone.name.clone(), reason });
            continue;
        }

        report.fixed.push(apply(&model, index, &mut out));
    }

    let doomed = doomed_bones(&model, &mut report);

    if doomed.is_empty() {
        return Ok((out, report));
    }

    let cut = remove_bones(&model, &out, &doomed)?;

    report.removed = doomed.iter().map(|&i| model.group_bones[i].name.clone()).collect();

    for fixed in &mut report.fixed {
        fixed.children.retain(|child| !report.removed.contains(child));
    }

    Ok((cut, report))
}

/// Whether a bone is one of the old item bones.
fn is_item_bone(name: &str) -> bool {
    let lower = name.to_lowercase();

    lower == "weapon" || lower == "offhand" || lower.starts_with("customhand")
}

/// The item bones that can come out: nothing hangs off them, they hold nothing, and no armature is attached to them. Those that cannot are reported.
fn doomed_bones(model: &Model, report: &mut Report) -> Vec<usize> {
    let mut doomed = Vec::new();

    for (index, bone) in model.group_bones.iter().enumerate() {
        if !is_item_bone(&bone.name) {
            continue;
        }

        let has_children = model.group_bones.iter().any(|other| other.parent_index == index as i16);
        let armature = model.armatures.iter().any(|armature| armature.parent_group_bone == index as i16);

        if has_children || !bone.elements.is_empty() || armature {
            report.skipped.push(Skipped {
                bone: bone.name.clone(),
                reason: "it is an old item bone, but something hangs off it or it holds cubes or an armature, so it is left in place".into(),
            });
            continue;
        }

        doomed.push(index);
    }

    doomed
}

/// The model's bytes with these bones cut out. The header count goes down, the bones after them move up, and every parent index (of a bone, or of an armature) is renumbered.
fn remove_bones(model: &Model, bytes: &[u8], doomed: &[usize]) -> Result<Vec<u8>, String> {
    let moved = |old: i16| -> i16 {
        if old < 0 {
            return old;
        }

        old - doomed.iter().filter(|&&gone| (gone as i16) < old).count() as i16
    };

    let mut out = Vec::with_capacity(bytes.len());
    let first = model.offsets.bones.first().ok_or("the model has no bones")?.start;
    let last = model.offsets.bones.last().ok_or("the model has no bones")?.end;

    out.extend_from_slice(&bytes[..first]);

    /* The number of bones, in the header (after the magic, version, flags, strings and textures: bytes 12 and 13). */
    let count = (model.group_bones.len() - doomed.len()) as u16;

    out[12..14].copy_from_slice(&count.to_le_bytes());

    for (index, at) in model.offsets.bones.iter().enumerate() {
        if doomed.contains(&index) {
            continue;
        }

        let start = out.len();

        out.extend_from_slice(&bytes[at.start..at.end]);

        let parent = moved(model.group_bones[index].parent_index);

        out[start + (at.parent - at.start)..start + (at.parent - at.start) + 2].copy_from_slice(&parent.to_le_bytes());
    }

    let tail_start = out.len();

    out.extend_from_slice(&bytes[last..]);

    for (armature, &at) in model.armatures.iter().zip(&model.offsets.armature_parents) {
        let parent = moved(armature.parent_group_bone);
        let place = tail_start + (at - last);

        out[place..place + 2].copy_from_slice(&parent.to_le_bytes());
    }

    Ok(out)
}

/// Why a bone cannot be fixed, or None.
fn refusal(model: &Model, index: usize) -> Option<String> {
    let bone = &model.group_bones[index];

    if bone.rotation[1].abs() > EPSILON || bone.rotation[2].abs() > EPSILON {
        return Some("it is turned on more than X, and only an X rotation is handled".into());
    }

    if bone.elements.iter().any(|element| element.kind == 0) {
        return Some("it has a mesh in it, and the rotation cannot be moved into a mesh here".into());
    }

    if bone.elements.is_empty() {
        return Some("it has no cubes to take the rotation, so zeroing it would turn its children".into());
    }

    for element in &bone.elements {
        let cube = &model.cubes[element.index as usize];

        if cube.rotation[1].abs() > EPSILON || cube.rotation[2].abs() > EPSILON {
            return Some(format!("its cube \"{}\" is turned on more than X, and only an X rotation is handled", cube.name));
        }
    }

    None
}

fn put(out: &mut [u8], at: usize, value: f64) {
    out[at..at + 4].copy_from_slice(&(value as f32).to_le_bytes());
}

fn put3(out: &mut [u8], at: usize, value: [f64; 3]) {
    for (axis, component) in value.iter().enumerate() {
        put(out, at + axis * 4, *component);
    }
}

fn apply(model: &Model, index: usize, out: &mut [u8]) -> Fixed {
    let bone = &model.group_bones[index];
    let at = &model.offsets.bones[index];
    let pivot = [bone.pivot[0] as f64, bone.pivot[1] as f64, bone.pivot[2] as f64];
    let turn = bone.rotation[0] as f64;
    let mut cubes = Vec::new();

    for element in &bone.elements {
        let cube = &model.cubes[element.index as usize];
        let cube_at = &model.offsets.cubes[element.index as usize];
        let origin = [cube.origin[0] as f64, cube.origin[1] as f64, cube.origin[2] as f64];
        let d = [pivot[0] - origin[0], pivot[1] - origin[1], pivot[2] - origin[2]];
        let shift = pivot_shift(d, cube.rotation[0] as f64);
        let net = wrap(turn + cube.rotation[0] as f64);

        put3(out, cube_at.origin, pivot);
        put3(out, cube_at.rotation, [net, 0.0, 0.0]);
        put3(out, cube_at.from, [cube.from[0] as f64 + shift[0], cube.from[1] as f64 + shift[1], cube.from[2] as f64 + shift[2]]);
        put3(out, cube_at.to, [cube.to[0] as f64 + shift[0], cube.to[1] as f64 + shift[1], cube.to[2] as f64 + shift[2]]);

        cubes.push(CubeChange { name: cube.name.clone(), old_rotation: cube.rotation[0], new_rotation: net as f32 });
    }

    put3(out, at.rotation, [0.0, 0.0, 0.0]);

    let children = model.group_bones.iter().filter(|other| other.parent_index == index as i16 && !is_item_bone(&other.name)).map(|other| other.name.clone()).collect();

    Fixed { bone: bone.name.clone(), old_rotation: bone.rotation[0], cubes, children }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Rotates a point about X by `degrees`, around `pivot`.
    fn turn_about(point: [f64; 3], pivot: [f64; 3], degrees: f64) -> [f64; 3] {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let y = point[1] - pivot[1];
        let z = point[2] - pivot[2];

        [point[0], pivot[1] + y * cos - z * sin, pivot[2] + y * sin + z * cos]
    }

    fn corners(from: [f64; 3], to: [f64; 3]) -> Vec<[f64; 3]> {
        let mut found = Vec::new();

        for x in [from[0], to[0]] {
            for y in [from[1], to[1]] {
                for z in [from[2], to[2]] {
                    found.push([x, y, z]);
                }
            }
        }

        found
    }

    #[test]
    fn the_numbers_agree_with_what_blockbench_does_when_a_pivot_moves() {
        /* A cube turned -35 about its pivot, whose pivot is moved up by one: the box moves by (0, 0.1808, -0.5735), as Blockbench shows. */
        let shift = pivot_shift([0.0, 1.0, 0.0], -35.0);

        assert!((shift[1] - 0.1808).abs() < 1e-3 && (shift[2] + 0.5736).abs() < 1e-3, "{shift:?}");

        /* And by (0, 0.8604, -0.2287) ... for a move of (-2, 0, 1.5): the box's X does not change, as the second screenshot shows. */
        let second = pivot_shift([-2.0, 0.0, 1.5], -35.0);

        assert!(second[0].abs() < 1e-9 && (second[1] - 0.8604).abs() < 1e-3 && (second[2] - (1.5 - 1.5 * 35f64.to_radians().cos())).abs() < 1e-3, "{second:?}");
    }

    #[test]
    fn the_arm_looks_the_same_after_it_is_fixed() {
        /* A bone turned 35 about its pivot with a cube turned -35 about another pivot, as in the game's models. */
        let pivot = [-3.3077, 10.47, -0.0017];
        let origin = [-2.2173, 8.5446, -2.7543];
        let (from, to) = ([-4.4016, 2.9243, -0.4891], [-2.217, 8.5449, 1.6787]);

        for (bone_turn, cube_turn) in [(35.0, -35.0), (140.0, -140.0), (-167.5, -140.0), (35.0, 25.0)] {
            let d = [pivot[0] - origin[0], pivot[1] - origin[1], pivot[2] - origin[2]];
            let shift = pivot_shift(d, cube_turn);
            let new_from = [from[0] + shift[0], from[1] + shift[1], from[2] + shift[2]];
            let new_to = [to[0] + shift[0], to[1] + shift[1], to[2] + shift[2]];
            let net = wrap(bone_turn + cube_turn);

            for (before, after) in corners(from, to).into_iter().zip(corners(new_from, new_to)) {
                let world_before = turn_about(turn_about(before, origin, cube_turn), pivot, bone_turn);
                let world_after = turn_about(after, pivot, net);

                for axis in 0..3 {
                    assert!((world_before[axis] - world_after[axis]).abs() < 1e-6, "bone {bone_turn}, cube {cube_turn}: {world_before:?} against {world_after:?}");
                }
            }
        }
    }

    #[test]
    fn wrapping_keeps_an_angle_in_range() {
        assert_eq!(wrap(0.0), 0.0);
        assert!((wrap(-307.5) - 52.5).abs() < 1e-9);
        assert!((wrap(200.0) + 160.0).abs() < 1e-9);
    }

    fn models_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../src/main/resources/assets/pleasurehorizons/strut/models")
    }

    /// Every lower arm in the game's own models, fixed: the bones come out at zero, the file reads back, and every cube is in the same place in the world as before.
    #[test]
    fn the_games_own_models_fix_without_moving_anything() {
        let folder = models_folder();

        if !folder.is_dir() {
            return;
        }

        let mut fixed_bones = 0;
        let mut stack = vec![folder];

        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);
                    continue;
                }

                if path.extension().map_or(true, |e| e != "strutmodel") {
                    continue;
                }

                let bytes = std::fs::read(&path).unwrap();
                let before = strut_model::read(&bytes).unwrap();
                let (out, report) = fix(&bytes).unwrap();
                let after = strut_model::read(&out).unwrap_or_else(|e| panic!("{} does not read back: {e}", path.display()));


                for fixed in &report.fixed {
                    let index = before.group_bones.iter().position(|b| b.name == fixed.bone).unwrap();
                    let bone_before = &before.group_bones[index];
                    let bone_after = after.group_bones.iter().find(|b| b.name == fixed.bone).unwrap();

                    assert_eq!(bone_after.rotation, [0.0, 0.0, 0.0], "{} {}", path.display(), fixed.bone);
                    fixed_bones += 1;

                    for element in &bone_before.elements {
                        let a = &before.cubes[element.index as usize];
                        let b = &after.cubes[element.index as usize];

                        assert_eq!(b.origin, bone_before.pivot, "the cube turns about the bone's pivot now");

                        let box_before = corners([a.from[0] as f64, a.from[1] as f64, a.from[2] as f64], [a.to[0] as f64, a.to[1] as f64, a.to[2] as f64]);
                        let box_after = corners([b.from[0] as f64, b.from[1] as f64, b.from[2] as f64], [b.to[0] as f64, b.to[1] as f64, b.to[2] as f64]);
                        let origin = [a.origin[0] as f64, a.origin[1] as f64, a.origin[2] as f64];
                        let pivot = [bone_before.pivot[0] as f64, bone_before.pivot[1] as f64, bone_before.pivot[2] as f64];

                        for (x, y) in box_before.into_iter().zip(box_after) {
                            let world_before = turn_about(turn_about(x, origin, a.rotation[0] as f64), pivot, bone_before.rotation[0] as f64);
                            let world_after = turn_about(y, pivot, b.rotation[0] as f64);

                            for axis in 0..3 {
                                assert!((world_before[axis] - world_after[axis]).abs() < 1e-3, "{} {} cube {}: {world_before:?} against {world_after:?}", path.display(), fixed.bone, a.name);
                            }
                        }
                    }
                }

                /* The old item bones are gone, everything else is still there with the same parent (by name), and the armatures still hang off the same bones. */
                assert!(after.group_bones.iter().all(|b| !is_item_bone(&b.name)), "{} still has an item bone", path.display());
                assert_eq!(after.group_bones.len(), before.group_bones.len() - report.removed.len());

                for bone in &after.group_bones {
                    let old = before.group_bones.iter().find(|b| b.name == bone.name).unwrap();
                    let parent_before = (old.parent_index >= 0).then(|| before.group_bones[old.parent_index as usize].name.clone());
                    let parent_after = (bone.parent_index >= 0).then(|| after.group_bones[bone.parent_index as usize].name.clone());

                    assert_eq!(parent_before, parent_after, "{}: {} changed parent", path.display(), bone.name);
                    assert_eq!(old.elements, bone.elements, "{}: {} changed its elements", path.display(), bone.name);
                }

                for (a, b) in before.armatures.iter().zip(&after.armatures) {
                    let name = |model: &Model, parent: i16| (parent >= 0).then(|| model.group_bones[parent as usize].name.clone());

                    assert_eq!(name(&before, a.parent_group_bone), name(&after, b.parent_group_bone), "{}: an armature moved", path.display());
                }

                assert_eq!(before.cubes.len(), after.cubes.len());
                assert_eq!(before.meshes.len(), after.meshes.len());
                assert_eq!(before.extensions, after.extensions);

                /* A second pass finds nothing left to do. */
                let (_, again) = fix(&out).unwrap();

                assert!(again.fixed.is_empty() && again.removed.is_empty(), "{} still has something to fix after fixing", path.display());
            }
        }

        assert!(fixed_bones > 0, "the game's models were expected to have arms of this kind");
    }
}
