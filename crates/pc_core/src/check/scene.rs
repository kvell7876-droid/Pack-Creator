//! A scene file: the settings every girl gets, who has it, and the settings a girl changes.

use super::values::check_object;
use crate::diagnostics::Sink;
use crate::json_ast::Node;
use crate::reference::Reference;

pub fn check(sink: &mut Sink, root: &Node, reference: &Reference) {
    let Some(members) = root.as_object() else {
        sink.error("wrong-type", format!("A scene file must be an object ({{ ... }}), not {}.", root.describe()), root.start, root.end);
        return;
    };

    /* girl_overrides is checked below: its entries are objects of the same settings. */
    check_object(sink, root, &reference.scene_fields, "a scene", &["girl_overrides"]);

    let supported = root.get("supported_girls");
    let unsupported = root.get("unsupported_girls");

    if let (Some(a), Some(b)) = (supported, unsupported) {
        sink.error("conflicting-fields", "supported_girls and unsupported_girls cannot be used together.", a.key_start, a.key_end);
        sink.error("conflicting-fields", "supported_girls and unsupported_girls cannot be used together.", b.key_start, b.key_end);
    }

    let override_fields: Vec<_> = reference.scene_fields.iter().filter(|f| !matches!(f.name.as_str(), "supported_girls" | "unsupported_girls" | "girl_overrides")).cloned().collect();

    if let Some(overrides) = root.get("girl_overrides") {
        let Some(entries) = overrides.value.as_object() else {
            sink.error("wrong-type", "\"girl_overrides\" must be an object with one entry for each girl.", overrides.value.start, overrides.value.end);
            return;
        };

        for entry in entries {
            check_object(sink, &entry.value, &override_fields, &format!("the override for {}", entry.key), &[]);
        }
    }

    if members.is_empty() {
        sink.info("empty-scene", "This scene sets nothing, so it is a scene with the default settings. Add a \"type\", such as \"on_bed\".", root.start, root.end);
    }

}
