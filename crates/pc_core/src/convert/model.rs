//! Bedrock `.geo.json` into Strut's binary model (`.strutmodel`). Cubes and bones only, which is everything the Pleasure Horizons models use; meshes, locators,
//! armatures and skinning are reported, not converted.
//!
//! Conventions, as the TypeScript converter has them (derived there from both of the game's bakers): the JSON baker negates X and the binary one does not, so the
//! converter writes bone pivot `(-x, y, z)`, bone rotation `(-x, -y, z)`, cube `from = (-(ox + sx), oy, oz)`, `to = (-ox, oy + sy, oz + sz)`, the cube's own pivot
//! `(-x, y, z)` and its rotation `(-x, -y, z)`; a box UV becomes six rectangles with the layout and floored sizes the JSON baker uses.

use super::findings::{Findings, Severity, Where};
use super::js::{negated, number_or};
use super::writer::{Out, Strings};
use serde_json::Value;
use std::collections::HashMap;

pub const MODEL_VERSION: u32 = 101;
const HEADER_BYTES: usize = 64;
const FLAG_EXTENSIONS: u32 = 1;
const MESH_FLAG_MIRROR: u32 = 2;
const MESH_FLAG_SKINNED: u32 = 4;
const MESH_FLAG_SMOOTH: u32 = 8;
const CUBE_FLAG_MIRROR: u32 = 2;
const CUBE_FLAG_BOX_UV: u32 = 4;
const ELEMENT_KIND_CUBE: u32 = 1;
pub const UNITS_PER_BLOCK: f64 = 16.0;

pub type Vec3 = [f64; 3];

#[derive(Debug, Clone)]
pub struct Texture {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub uv_width: f64,
    pub uv_height: f64,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub kind: u32,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct GroupBone {
    pub name: String,
    pub parent: i32,
    pub pivot: Vec3,
    pub rotation: Vec3,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct CubeFace {
    pub face_id: u32,
    pub u1: f64,
    pub v1: f64,
    pub u2: f64,
    pub v2: f64,
    pub uv_rotation: i64,
}

#[derive(Debug, Clone)]
pub struct Cube {
    pub name: String,
    pub texture: u32,
    pub origin: Vec3,
    pub rotation: Vec3,
    pub from: Vec3,
    pub to: Vec3,
    pub inflate: f64,
    pub mirror: bool,
    pub box_uv: bool,
    pub faces: Vec<CubeFace>,
}

/// A model as the binary format holds it. Meshes and armatures are not produced by the converter, so they are not here.
#[derive(Debug, Clone)]
pub struct RawModel {
    pub model_id: Option<String>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub strings: Vec<String>,
    pub textures: Vec<Texture>,
    pub group_bones: Vec<GroupBone>,
    pub cubes: Vec<Cube>,
    /// Named extension sections (physics), in the order they are written.
    pub extensions: Vec<(String, Vec<u8>)>,
}

pub struct Converted {
    pub model: RawModel,
    pub findings: Findings,
}

// =================================================================
// Encoding
// =================================================================

pub fn encode(model: &RawModel) -> Result<Vec<u8>, String> {
    let mut strings = Strings::default();

    for s in &model.strings {
        strings.intern(s);
    }

    let table = strings.write()?;
    let mut textures = Out::default();

    for t in &model.textures {
        textures.u16(strings.index_of(&t.name, "texture name")?).u16(t.width as u32).u16(t.height as u32).u16(t.uv_width as u32).u16(t.uv_height as u32);
    }

    let mut bones = Out::default();

    for bone in &model.group_bones {
        bones.u16(strings.index_of(&bone.name, "bone name")?).i16(bone.parent).vec3(bone.pivot).vec3(bone.rotation).u8(0).u16(bone.elements.len() as u32);

        for element in &bone.elements {
            bones.u8(element.kind).u16(element.index);
        }
    }

    /* The converter makes no meshes. */
    let meshes = Out::default();
    let mut cubes = Out::default();

    for cube in &model.cubes {
        let mut flags = 0;

        if cube.mirror {
            flags |= CUBE_FLAG_MIRROR;
        }

        if cube.box_uv {
            flags |= CUBE_FLAG_BOX_UV;
        }

        cubes.u16(strings.index_of(&cube.name, "cube name")?).u16(cube.texture).vec3(cube.origin).vec3(cube.rotation).vec3(cube.from).vec3(cube.to).f32(cube.inflate).u8(flags).u8(cube.faces.len() as u32);

        for face in &cube.faces {
            cubes.u8(face.face_id).f32(face.u1).f32(face.v1).f32(face.u2).f32(face.v2).u8((face.uv_rotation & 0xFF) as u32);
        }
    }

    let armatures = Out::default();
    let mut extensions = Out::default();

    if !model.extensions.is_empty() {
        extensions.u16(model.extensions.len() as u32);

        for (name, payload) in &model.extensions {
            extensions.string(name)?.u32(payload.len() as u32).bytes(payload);
        }
    }

    let mut header = Out::default();

    header.bytes(b"STMD").u16(MODEL_VERSION).u16(if model.extensions.is_empty() { 0 } else { FLAG_EXTENSIONS }).u16(model.strings.len() as u32).u16(model.textures.len() as u32).u16(model.group_bones.len() as u32).u16(0).u16(model.cubes.len() as u32).u16(0).u32(table.len() as u32).f32(UNITS_PER_BLOCK).vec3(model.bounds_min).vec3(model.bounds_max).zeros(12);

    if header.len() != HEADER_BYTES {
        return Err(format!("wrote a {}-byte model header, expected {HEADER_BYTES}", header.len()));
    }

    let mut out = Out::default();

    for part in [&header, &table, &textures, &bones, &meshes, &cubes, &armatures, &extensions] {
        out.bytes(&part.0);
    }

    let _ = (MESH_FLAG_MIRROR, MESH_FLAG_SKINNED, MESH_FLAG_SMOOTH);
    Ok(out.0)
}

// =================================================================
// Skeleton fingerprint
// =================================================================

/// `java.lang.String.hashCode()` over UTF-16 code units, at 32-bit signed precision.
fn java_hash(text: &str) -> i32 {
    text.encode_utf16().fold(0i32, |hash, unit| hash.wrapping_mul(31).wrapping_add(unit as i32))
}

/// The hash the game recomputes from a skeleton: every group bone's name in order, each followed by a NUL, hashed as one string.
pub fn fingerprint(model: &RawModel) -> i32 {
    let mut names = String::new();

    for bone in &model.group_bones {
        names.push_str(&bone.name);
        names.push('\0');
    }

    java_hash(&names)
}

// =================================================================
// Conversion
// =================================================================

const FACES: [&str; 6] = ["west", "east", "north", "south", "up", "down"];
const KNOWN_BONE_KEYS: [&str; 9] = ["name", "parent", "pivot", "rotation", "cubes", "mirror", "inflate", "neverRender", "reset"];
const KNOWN_CUBE_KEYS: [&str; 7] = ["origin", "size", "pivot", "rotation", "uv", "inflate", "mirror"];

fn vec3(value: Option<&Value>) -> Vec3 {
    match value.and_then(Value::as_array) {
        Some(list) => [number_or(list.first(), 0.0), number_or(list.get(1), 0.0), number_or(list.get(2), 0.0)],
        None => [0.0; 3],
    }
}

fn mirror_x(v: Vec3) -> Vec3 {
    [negated(v[0]), v[1], v[2]]
}

fn negate_xy(v: Vec3) -> Vec3 {
    [negated(v[0]), negated(v[1]), v[2]]
}

fn empty_model() -> RawModel {
    RawModel { model_id: None, bounds_min: [0.0; 3], bounds_max: [0.0; 3], strings: Vec::new(), textures: Vec::new(), group_bones: Vec::new(), cubes: Vec::new(), extensions: Vec::new() }
}

/// Converts a legacy `.geo.json` (format 1.12.0 or 1.21.0).
pub fn convert_geo(root: &Value, file: Option<&str>) -> Converted {
    let mut findings = Findings::default();
    let here = Where { file: file.map(String::from), ..Default::default() };

    let geometries = root.get("minecraft:geometry").and_then(Value::as_array).filter(|g| !g.is_empty());
    let Some(geometries) = geometries else {
        findings.add(Severity::Lossy, "no \"minecraft:geometry\" array (only format 1.12.0 and 1.21.0 files are read); nothing converted", &here);

        return Converted { model: empty_model(), findings };
    };

    if geometries.len() > 1 {
        findings.add(Severity::Lossy, format!("{} geometries in one file; only the first is converted", geometries.len()), &here);
    }

    let geometry = &geometries[0];
    let description = geometry.get("description");
    let field = |name: &str| description.and_then(|d| d.get(name));
    let mut strings = Strings::default();
    let identifier = field("identifier").and_then(Value::as_str).map(String::from);
    let texture_width = number_or(field("texture_width"), 16.0);
    let texture_height = number_or(field("texture_height"), 16.0);
    let texture_name = match &identifier {
        Some(id) => id.strip_prefix("geometry.").unwrap_or(id).to_string(),
        None => "texture".to_string(),
    };

    if let Some(id) = &identifier {
        strings.intern(id);
    }

    strings.intern(&texture_name);

    let source: Vec<&Value> = geometry.get("bones").and_then(Value::as_array).map(|b| b.iter().collect()).unwrap_or_default();
    let ordered = order_bones(&source, &mut findings, file);
    let mut index_by_name: HashMap<String, i32> = HashMap::new();
    let mut group_bones: Vec<GroupBone> = Vec::new();
    let mut cubes: Vec<Cube> = Vec::new();

    for bone in ordered {
        let name = bone.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let at = Where { file: file.map(String::from), bone: Some(name.clone()), ..Default::default() };

        if let Some(object) = bone.as_object() {
            for key in object.keys() {
                if !KNOWN_BONE_KEYS.contains(&key.as_str()) {
                    findings.add(Severity::Lossy, format!("bone field \"{key}\" is not carried over"), &at);
                }
            }
        }

        if bone.get("reset").is_some() {
            findings.add(Severity::Lossy, "bone field \"reset\" is not carried over", &at);
        }

        if bone.get("neverRender") == Some(&Value::Bool(true)) {
            findings.add(Severity::Lossy, "bone field \"neverRender\" is not carried over: a model no longer says what is hidden; hide the bone in the game with an override", &at);
        }

        if bone.get("mirror") == Some(&Value::Bool(true)) {
            findings.add(Severity::Info, "bone-level \"mirror\" is ignored, as the old JSON loader did (only a cube's own mirror counts)", &at);
        }

        strings.intern(&name);

        let mut elements = Vec::new();
        let bone_cubes: Vec<&Value> = bone.get("cubes").and_then(Value::as_array).map(|c| c.iter().collect()).unwrap_or_default();

        for (i, cube) in bone_cubes.iter().enumerate() {
            let raw = convert_cube(cube, &format!("{name}_{i}"), bone, &mut strings, &mut findings, &at);

            elements.push(Element { kind: ELEMENT_KIND_CUBE, index: cubes.len() as u32 });
            cubes.push(raw);
        }

        let parent = bone.get("parent").and_then(Value::as_str).and_then(|p| index_by_name.get(p)).copied().unwrap_or(-1);

        index_by_name.insert(name.clone(), group_bones.len() as i32);
        group_bones.push(GroupBone { name, parent, pivot: mirror_x(vec3(bone.get("pivot"))), rotation: negate_xy(vec3(bone.get("rotation"))), elements });
    }

    let width = number_or(field("visible_bounds_width"), 0.0);
    let height = number_or(field("visible_bounds_height"), 0.0);
    let offset = vec3(field("visible_bounds_offset"));

    Converted {
        model: RawModel {
            model_id: identifier,
            bounds_min: [-offset[0] - width / 2.0, offset[1] - height / 2.0, offset[2] - width / 2.0],
            bounds_max: [-offset[0] + width / 2.0, offset[1] + height / 2.0, offset[2] + width / 2.0],
            strings: strings.entries().to_vec(),
            textures: vec![Texture { name: texture_name, width: texture_width, height: texture_height, uv_width: texture_width, uv_height: texture_height }],
            group_bones,
            cubes,
            extensions: Vec::new(),
        },
        findings,
    }
}

/// Parents first, depth first, children in the order the file lists them. A bone whose parent does not exist becomes a root (reported), a second bone of the same name is
/// dropped (reported), and a cycle is broken (reported).
fn order_bones<'a>(bones: &[&'a Value], findings: &mut Findings, file: Option<&str>) -> Vec<&'a Value> {
    let here = Where { file: file.map(String::from), ..Default::default() };
    let mut by_name: Vec<(&str, &Value)> = Vec::new();

    for bone in bones {
        let Some(name) = bone.get("name").and_then(Value::as_str) else {
            findings.add(Severity::Lossy, "a bone with no name is skipped", &here);
            continue;
        };

        if by_name.iter().any(|(n, _)| *n == name) {
            findings.add(Severity::Lossy, format!("a second bone named \"{name}\"; only the first is kept"), &Where { bone: Some(name.to_string()), ..here.clone() });
            continue;
        }

        by_name.push((name, bone));
    }

    let mut children: HashMap<&str, Vec<&Value>> = HashMap::new();
    let mut roots: Vec<&Value> = Vec::new();

    for (name, bone) in &by_name {
        let parent = bone.get("parent").and_then(Value::as_str);

        match parent {
            Some(p) if p != *name && by_name.iter().any(|(n, _)| *n == p) => children.entry(p).or_default().push(bone),
            _ => {
                if let Some(p) = parent {
                    findings.add(Severity::Lossy, format!("its parent \"{p}\" does not exist; it becomes a top-level bone"), &Where { bone: Some(name.to_string()), ..here.clone() });
                }

                roots.push(bone);
            }
        }
    }

    let mut ordered: Vec<&Value> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();

    fn visit<'a>(bone: &'a Value, children: &HashMap<&str, Vec<&'a Value>>, seen: &mut Vec<&'a str>, ordered: &mut Vec<&'a Value>) {
        let name = bone.get("name").and_then(Value::as_str).unwrap_or("");

        if seen.contains(&name) {
            return;
        }

        seen.push(name);
        ordered.push(bone);

        for child in children.get(name).map(Vec::as_slice).unwrap_or(&[]) {
            visit(child, children, seen, ordered);
        }
    }

    for root in roots {
        visit(root, &children, &mut seen, &mut ordered);
    }

    for (name, bone) in &by_name {
        if !seen.contains(name) {
            findings.add(Severity::Lossy, "its parent chain loops back on itself; it becomes a top-level bone", &Where { bone: Some(name.to_string()), ..here.clone() });
            visit(bone, &children, &mut seen, &mut ordered);
        }
    }

    ordered
}

fn convert_cube(cube: &Value, name: &str, bone: &Value, strings: &mut Strings, findings: &mut Findings, at: &Where) -> Cube {
    if let Some(object) = cube.as_object() {
        for key in object.keys() {
            if !KNOWN_CUBE_KEYS.contains(&key.as_str()) {
                findings.add(Severity::Lossy, format!("cube field \"{key}\" is not carried over"), at);
            }
        }
    }

    strings.intern(name);

    let origin = vec3(cube.get("origin"));
    let size = vec3(cube.get("size"));
    let box_uv = cube.get("uv").map_or(false, Value::is_array);
    let mirror = cube.get("mirror") == Some(&Value::Bool(true));

    if box_uv && mirror {
        findings.add(Severity::Lossy, "a mirrored cube with a box UV: the binary format draws its top and bottom faces swapped; give it per-face UVs in Blockbench", at);
    }

    Cube {
        name: name.to_string(),
        texture: 0,
        origin: mirror_x(vec3(cube.get("pivot"))),
        rotation: negate_xy(vec3(cube.get("rotation"))),
        from: [negated(origin[0] + size[0]), origin[1], origin[2]],
        to: [negated(origin[0]), origin[1] + size[1], origin[2] + size[2]],
        inflate: number_or(cube.get("inflate"), number_or(bone.get("inflate"), 0.0)),
        mirror,
        box_uv,
        faces: if box_uv { box_faces(cube.get("uv").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]), size) } else { per_face_faces(cube.get("uv"), findings, at) },
    }
}

/// The six rectangles of a Bedrock box UV, with the layout and the floored sizes the JSON baker uses.
fn box_faces(uv: &[Value], size: Vec3) -> Vec<CubeFace> {
    let (u, v) = (number_or(uv.first(), 0.0), number_or(uv.get(1), 0.0));
    let (sx, sy, sz) = (size[0].floor(), size[1].floor(), size[2].floor());
    let rects: [[f64; 4]; 6] = [
        [u + sz + sx, v + sz, sz, sy],
        [u, v + sz, sz, sy],
        [u + sz, v + sz, sx, sy],
        [u + sz + sx + sz, v + sz, sx, sy],
        [u + sz, v, sx, sz],
        [u + sz + sx, v + sz, sx, -sz],
    ];

    rects.iter().enumerate().map(|(face_id, [u1, v1, w, h])| CubeFace { face_id: face_id as u32, u1: *u1, v1: *v1, u2: u1 + w, v2: v1 + h, uv_rotation: 0 }).collect()
}

fn per_face_faces(uv: Option<&Value>, findings: &mut Findings, at: &Where) -> Vec<CubeFace> {
    let Some(object) = uv.and_then(Value::as_object) else { return Vec::new() };

    for direction in object.keys() {
        if !FACES.contains(&direction.as_str()) {
            findings.add(Severity::Lossy, format!("UV face \"{direction}\" is not one of west, east, north, south, up, down; skipped"), at);
        }
    }

    let mut faces = Vec::new();

    for (face_id, direction) in FACES.iter().enumerate() {
        let Some(face) = object.get(*direction).filter(|f| !f.is_null()) else { continue };
        let origin = face.get("uv").and_then(Value::as_array);
        let extent = face.get("uv_size").and_then(Value::as_array);
        let u1 = number_or(origin.and_then(|o| o.first()), 0.0);
        let v1 = number_or(origin.and_then(|o| o.get(1)), 0.0);
        let degrees = number_or(face.get("uv_rotation"), 0.0);

        faces.push(CubeFace {
            face_id: face_id as u32,
            u1,
            v1,
            u2: u1 + number_or(extent.and_then(|e| e.first()), 0.0),
            v2: v1 + number_or(extent.and_then(|e| e.get(1)), 0.0),
            /* Whole 90 degree steps, wrapped the way JavaScript's % does (a negative stays negative). */
            uv_rotation: (degrees / 90.0).floor() as i64 % 4,
        });
    }

    faces
}
