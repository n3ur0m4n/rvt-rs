//! Family-instance bodies from the meshes Revit saves for display (#255).
//!
//! A Revit file keeps the graphics it last drew for each element. rosejn's
//! native reader (#255: `native_saved_scene`, `native_saved_mesh` and the
//! schema, graph and identity code under them) decodes them in document
//! feet. Where rvt-rs has only an element record's bounding box, as for
//! doors, windows and furniture, [`attach`] replaces that box with the
//! element's saved mesh.
//!
//! On RE1's four models (Revit 2025) 207 of the 214 bodies replaced are
//! within 0.01 ft of Revit's export (B77, Measure run 37231768480).
//!
//! On Core Interior the saved meshes coincide with the bodies rvt-rs already
//! draws exactly (walls 351 of 360, columns 256 of 256, slabs 80 of 80), and
//! a door's mesh is its panel and frame, 3.50 × 0.83 × 8.25 ft, where its
//! record box is 3.75 ft deep because it includes the swing: the 2.92 ft
//! difference #227 measured against Revit's own export.
//!
//! Reading saved graphics is slow on large files, so it is opt-in
//! (`rvt-ifc --saved-meshes`, `rvt-gltf --saved-meshes`).

use super::IfcModel;
use super::entities::{BrepTriangle, IfcEntity, Property, PropertySet, PropertyValue, SolidShape};
use super::export_content::RECORD_BBOX_BODY_SOURCE;
use std::collections::{BTreeMap, BTreeSet};

/// `BodySource` of a body drawn from the element's saved graphics.
pub const SAVED_MESH_BODY_SOURCE: &str = "saved_graphics_mesh";

/// What [`attach`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SavedMeshReport {
    /// Elements whose body was the record's bounding box.
    pub candidates: usize,
    /// Of those, elements now drawn from their saved mesh.
    pub replaced: usize,
    /// Triangles written in all.
    pub triangles: usize,
}

/// Replace each bounding-box body with the element's saved mesh, where the
/// file holds one.
pub fn attach(rf: &mut crate::RevitFile, model: &mut IfcModel) -> anyhow::Result<SavedMeshReport> {
    let mut report = SavedMeshReport::default();
    let mut by_id: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    // A stair or curtain wall is drawn by its parts; its own saved graphics
    // would draw it twice.
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            location_feet: Some(_),
            ..
        } = entity
        else {
            continue;
        };
        let text = |name: &str| {
            set.properties.iter().find_map(|p| match &p.value {
                PropertyValue::Text(t) if p.name == name => Some(t.as_str()),
                _ => None,
            })
        };
        let profile_resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        if text("BodySource") != Some(RECORD_BBOX_BODY_SOURCE) || profile_resolved {
            continue;
        }
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        report.candidates += 1;
        by_id.entry(id).or_default().push(index);
    }
    if by_id.is_empty() {
        return Ok(report);
    }
    let options = crate::native_document::Options {
        selected_ids: by_id.keys().copied().collect::<BTreeSet<_>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = crate::native_saved_scene::extract_at_detail(rf, &options, 3)?;
    for element in &scene.elements {
        let Some(indices) = by_id.get(&element.identity.element_id) else {
            continue;
        };
        // A Tag carried by several entities (a slab's pieces) is ambiguous:
        // leave them as they are.
        let [index] = indices.as_slice() else {
            continue;
        };
        let primitives = &element.meshes.primitives;
        if primitives.is_empty() {
            continue;
        }
        let IfcEntity::BuildingElement {
            location_feet: Some(origin),
            rotation_radians,
            solid_shape,
            property_set: Some(set),
            ..
        } = &mut model.entities[*index]
        else {
            continue;
        };
        let (sin, cos) = (-rotation_radians.unwrap_or(0.0)).sin_cos();
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for primitive in primitives {
            let base = vertices.len() as u32;
            vertices.extend(primitive.vertices.iter().map(|p| {
                let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
                [dx * cos - dy * sin, dx * sin + dy * cos, p[2] - origin[2]]
            }));
            triangles.extend(
                primitive
                    .triangles
                    .iter()
                    .filter(|t| t.iter().all(|&v| (v as usize) < primitive.vertices.len()))
                    .map(|t| BrepTriangle(base + t[0], base + t[1], base + t[2])),
            );
        }
        if triangles.is_empty() {
            continue;
        }
        report.replaced += 1;
        report.triangles += triangles.len();
        *solid_shape = Some(SolidShape::TriangulatedFaceSet {
            vertices_feet: vertices,
            triangles,
        });
        for property in &mut set.properties {
            if property.name == "BodySource" {
                property.value = PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into());
            }
        }
    }
    Ok(report)
}

/// What [`attach_all`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SavedSceneReport {
    /// Saved-graphics records with triangles.
    pub with_mesh: usize,
    /// Of those, the ones drawn: not types, and owned by nothing or by a
    /// family instance that is drawn itself.
    pub drawable: usize,
    /// Bodies of elements rvt-rs already exported, replaced.
    pub replaced: usize,
    /// Elements rvt-rs did not export, added with their saved mesh only.
    pub added: usize,
    /// Drawable elements left as they were: their Tag is on several
    /// entities, or on the whole of an aggregate.
    pub ambiguous: usize,
    /// Triangles written in all.
    pub triangles: usize,
}

/// A record class that names an element type: a family or one of its types.
fn is_type_class(class: &str) -> bool {
    class == "Family" || class.ends_with("Symbol") || class.ends_with("Type")
}

/// Draw every element from the meshes the file saved for display, where it
/// has one, in place of the body rvt-rs builds; elements rvt-rs does not
/// export are added with that mesh as their only body.
///
/// The saved graphics hold types and the parts of nested families too. An
/// element is drawn when it is not a type and is owned by nothing or by a
/// drawn family instance: on the four real models (2024-2026) this draws
/// every element of the add-in geometry that has a saved mesh, and nothing
/// else (2026-10-20).
pub fn attach_all(
    rf: &mut crate::RevitFile,
    model: &mut IfcModel,
) -> anyhow::Result<SavedSceneReport> {
    let scene = saved_scene(rf)?;
    attach_scene(rf, model, scene)
}

fn scene_options() -> crate::native_document::Options {
    crate::native_document::Options {
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    }
}

/// The saved meshes [`attach_all`] draws, read on their own: a caller can
/// read them from a second handle while it exports the model.
pub fn saved_scene(
    rf: &mut crate::RevitFile,
) -> anyhow::Result<crate::native_saved_scene::SavedScene> {
    crate::native_saved_scene::extract_at_detail(rf, &scene_options(), 3)
}

/// [`attach_all`] with the scene already read by [`saved_scene`].
pub fn attach_scene(
    rf: &mut crate::RevitFile,
    model: &mut IfcModel,
    scene: crate::native_saved_scene::SavedScene,
) -> anyhow::Result<SavedSceneReport> {
    let mut report = SavedSceneReport::default();
    let options = scene_options();
    let with_mesh: Vec<_> = scene
        .elements
        .iter()
        .filter(|e| e.meshes.primitives.iter().any(|p| !p.triangles.is_empty()))
        .collect();
    report.with_mesh = with_mesh.len();

    // Class and owner of each element and of its owners, up the chain.
    let mut owner: BTreeMap<u64, i64> = BTreeMap::new();
    let mut class: BTreeMap<u64, String> = BTreeMap::new();
    let mut wanted: BTreeSet<u64> = with_mesh.iter().map(|e| e.identity.element_id).collect();
    for _ in 0..16 {
        if wanted.is_empty() {
            break;
        }
        let mut opts = options.clone();
        opts.selected_ids = wanted.clone();
        crate::native_document::extract_graphs(rf, &opts, |r| {
            owner.insert(r.identity.element_id, r.identity.owning_element_id);
            if let Some(c) = &r.class_name {
                class.insert(r.identity.element_id, c.clone());
            }
            Ok(())
        })?;
        for id in &wanted {
            owner.entry(*id).or_insert(-1);
        }
        wanted = owner
            .values()
            .filter_map(|o| u64::try_from(*o).ok().filter(|o| *o > 0))
            .filter(|o| !owner.contains_key(o))
            .collect();
    }
    let class_of = |id: u64| class.get(&id).map(String::as_str).unwrap_or("");
    let drawable = |mut id: u64| {
        for _ in 0..16 {
            if is_type_class(class_of(id)) {
                return false;
            }
            match u64::try_from(owner.get(&id).copied().unwrap_or(-1)) {
                Ok(o) if o > 0 => {
                    if class_of(o) != "FamilyInstance" {
                        return false;
                    }
                    id = o;
                }
                _ => return true,
            }
        }
        false
    };

    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    let mut by_id: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            ..
        } = entity
        {
            if let Ok(id) = tag.parse::<u64>() {
                by_id.entry(id).or_default().push(index);
            }
        }
    }
    for element in with_mesh {
        let id = element.identity.element_id;
        if !drawable(id) {
            continue;
        }
        report.drawable += 1;
        let primitives = &element.meshes.primitives;
        match by_id.get(&id).map(Vec::as_slice) {
            Some([index]) if !wholes.contains(index) => {
                let IfcEntity::BuildingElement {
                    location_feet,
                    rotation_radians,
                    solid_shape,
                    representation_map_index,
                    property_set,
                    ..
                } = &mut model.entities[*index]
                else {
                    continue;
                };
                let shape = local_mesh(primitives, *location_feet, *rotation_radians);
                report.triangles += triangle_count(&shape);
                *solid_shape = Some(shape);
                *representation_map_index = None;
                if let Some(set) = property_set {
                    for property in &mut set.properties {
                        if property.name == "BodySource" {
                            property.value = PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into());
                        }
                    }
                }
                report.replaced += 1;
            }
            Some(_) => report.ambiguous += 1,
            None => {
                let revit_class = class_of(id);
                let ifc_type = super::category_map::lookup(revit_class)
                    .map(|m| m.ifc_type)
                    .unwrap_or("IFCBUILDINGELEMENTPROXY");
                let shape = local_mesh(primitives, None, None);
                report.triangles += triangle_count(&shape);
                model.entities.push(IfcEntity::BuildingElement {
                    ifc_type: ifc_type.into(),
                    name: format!("{revit_class}:{id}"),
                    type_guid: Some(id.to_string()),
                    predefined_type: None,
                    storey_index: None,
                    material_index: None,
                    property_set: Some(PropertySet {
                        name: format!("Pset_RevitType_{revit_class}"),
                        properties: vec![Property {
                            name: "BodySource".into(),
                            value: PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into()),
                        }],
                    }),
                    location_feet: None,
                    rotation_radians: None,
                    extrusion: None,
                    host_element_index: None,
                    material_layer_set_index: None,
                    material_profile_set_index: None,
                    solid_shape: Some(shape),
                    representation_map_index: None,
                });
                report.added += 1;
            }
        }
    }
    // Freeing the decoded graphs takes seconds on large models: do it aside.
    std::thread::spawn(move || drop(scene));
    Ok(report)
}

/// The saved mesh in the element frame: document feet less `location`,
/// turned back by `rotation` about +Z.
fn local_mesh(
    primitives: &[crate::native_saved_mesh::Primitive],
    location: Option<[f64; 3]>,
    rotation: Option<f64>,
) -> SolidShape {
    let origin = location.unwrap_or([0.0; 3]);
    let (sin, cos) = (-rotation.unwrap_or(0.0)).sin_cos();
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for primitive in primitives {
        let base = vertices.len() as u32;
        vertices.extend(primitive.vertices.iter().map(|p| {
            let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
            [dx * cos - dy * sin, dx * sin + dy * cos, p[2] - origin[2]]
        }));
        triangles.extend(
            primitive
                .triangles
                .iter()
                .filter(|t| t.iter().all(|&v| (v as usize) < primitive.vertices.len()))
                .map(|t| BrepTriangle(base + t[0], base + t[1], base + t[2])),
        );
    }
    SolidShape::TriangulatedFaceSet {
        vertices_feet: vertices,
        triangles,
    }
}

fn triangle_count(shape: &SolidShape) -> usize {
    match shape {
        SolidShape::TriangulatedFaceSet { triangles, .. } => triangles.len(),
        _ => 0,
    }
}
