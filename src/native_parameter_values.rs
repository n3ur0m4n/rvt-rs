//! Parameter values of one native record, as the Revit API reports them.
//!
//! Sources, in order of precedence for the same parameter id: the element's
//! saved parameter sets (`m_pParamValueSet*`), the directly projected saved
//! fields, then the family parameter slots of a family type. Values stay in
//! internal units, as serialized.
//!
//! On top of the saved values, a record reports what the API derives from
//! the element's own references and class, each measured against API exports
//! of the same files (Revit 2024-2027, real and synthetic corpus):
//!
//! - the type id (`ELEM_FAMILY_AND_TYPE_PARAM`, `ELEM_FAMILY_PARAM`,
//!   `ELEM_TYPE_PARAM`, `SYMBOL_ID_PARAM`) from the root field that holds
//!   it, which differs by class (`m_WallAttributesId`, `m_attrId`, ...);
//! - the category (`ELEM_CATEGORY_PARAM`, `ELEM_CATEGORY_PARAM_MT`) from the
//!   class table ([`crate::native_class_categories`]) or, for family
//!   symbols, instances and floors, from the family or type they reference
//!   ([`resolve`], second pass over every record);
//! - the type name from `SymbolInfo.m_name`, the family name of a family
//!   symbol from its `Family`, the datum name (`DATUM_TEXT`) from `m_text`;
//! - `DESIGN_OPTION_ID` from `m_designOptionId`, and the IFC export flags and
//!   identifiers the API reports when nothing is saved: `0` for the export
//!   flags, the identifier derived from the UniqueId for the GUIDs (both
//!   verified on every element of the corpus).
//!
//! Some serialized values are not the value the API reports, because Revit
//! derives the parameter at runtime: those values are withheld, `value` is
//! `None` and `diagnostic` says why. So are the names the API localizes
//! (system family names, empty saved type names). Nothing is evaluated here.

use crate::native_class_categories::{
    class_category, class_exposes_ifc_export, family_category_exposes_ifc_export,
};
use crate::native_document::Record;
use crate::native_metadata::identifier;
use crate::native_parameters::{GraphObject, ObjectGraph};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// `SYMBOL_FAMILY_NAME_PARAM` before Revit 2026.
const ALL_MODEL_FAMILY_NAME: i64 = -1002002;
/// `SYMBOL_NAME_PARAM` before Revit 2026.
const ALL_MODEL_TYPE_NAME: i64 = -1002001;
const ASSEMBLY_DESCRIPTION: i64 = -1002501;
const DATUM_TEXT: i64 = -1008000;
const DESIGN_OPTION_ID: i64 = -1013201;
const ELEM_CATEGORY_PARAM: i64 = -1140362;
const ELEM_CATEGORY_PARAM_MT: i64 = -1140363;
const ELEM_FAMILY_AND_TYPE_PARAM: i64 = -1002052;
const ELEM_FAMILY_PARAM: i64 = -1002051;
const ELEM_TYPE_PARAM: i64 = -1002050;
const HEAVY_END_PEN: i64 = -1006447;
const HOST_ID_PARAM: i64 = -1002108;
const IFC_EXPORT_ELEMENT: i64 = -1019012;
const IFC_EXPORT_ELEMENT_TYPE: i64 = -1019013;
const IFC_GUID: i64 = -1019000;
const IFC_TYPE_GUID: i64 = -1019001;
const PHY_MATERIAL_PARAM_UNIT_WEIGHT: i64 = -1140309;
const SPOT_DIM_LEADER: i64 = -1006466;
const SYMBOL_ID_PARAM: i64 = -1002000;
const WALL_HEIGHT_TYPE: i64 = -1001103;
const WALL_USER_HEIGHT_PARAM: i64 = -1001105;
const ZONE_NAME: i64 = -1114300;

const OST_FLOORS: i64 = -2000032;
const OST_STRUCTURAL_FOUNDATION: i64 = -2001300;

/// The four parameters that report an instance's type id.
const TYPE_ID_PARAMS: [i64; 4] = [
    ELEM_FAMILY_AND_TYPE_PARAM,
    ELEM_FAMILY_PARAM,
    ELEM_TYPE_PARAM,
    SYMBOL_ID_PARAM,
];
/// Root field holding the element's type id, per native class. Measured: the
/// four type parameters of every instance of the class equal this field.
/// `FamilyInstance` (`m_masterSymbolId`) is projected by `native_metadata`.
const TYPE_ID_FIELDS: &[(&str, &str)] = &[
    ("Alignment", "m_styleSymbolId"),
    ("AngularDim", "m_styleSymbolId"),
    ("DBView3d", "m_dbViewTypeId"),
    ("DBViewAnonDrafting", "m_dbViewTypeId"),
    ("DBViewDrafting", "m_dbViewTypeId"),
    ("DBViewLegend", "m_dbViewTypeId"),
    ("DBViewPlan", "m_dbViewTypeId"),
    ("DBViewSchedule", "m_dbViewTypeId"),
    ("DBViewSection", "m_dbViewTypeId"),
    ("FabricationPart", "m_idType"),
    ("FilledRegion", "m_attrId"),
    ("Floor", "m_floorAttributesId"),
    ("Grid", "m_attrId"),
    ("IndependentTag", "m_symbolId"),
    ("InteriorElev", "m_symbolId"),
    ("Level", "m_attrId"),
    ("LinearDimString", "m_styleSymbolId"),
    ("ProfileRoof", "m_roofAttributesId"),
    ("RadialDim", "m_styleSymbolId"),
    ("RbsPipeCurve", "m_idType"),
    ("RbsPipingSystem", "m_typeId"),
    ("RebarSystem", "m_symbolId"),
    ("SWall", "m_WallAttributesId"),
    ("SpanDirectionSymbol", "m_symbolId"),
    ("SpotElevation", "m_styleSymbolId"),
    ("SweepHost", "m_typeId"),
];
/// Type id held by a field of the one object of this class in the graph.
const TYPE_ID_OBJECTS: &[(&str, &str, &str)] = &[
    ("GeoLocation", "InstanceInfo", "m_symbolId"),
    ("ImportInstance", "InstanceInfo", "m_symbolId"),
    ("PointCloudInstance", "InstanceInfo", "m_symbolId"),
    ("Rebar", "RebarInstance", "m_barTypeId"),
    ("RvtLinkInstance", "InstanceInfo", "m_symbolId"),
];
/// Compact parameter stores: `(class, object holding the store, field)`. A
/// rebar saves its parameters as a map from parameter id to `(index,
/// storage)` over typed value arrays, in `RebarInstance.m_params` of a bar
/// and `m_instanceParams` of a shape (the root object itself).
const PARAMETER_STORES: &[(&str, &str, &str)] = &[
    ("Rebar", "RebarInstance", "m_params"),
    ("RebarShape", "RebarShape", "m_instanceParams"),
];
/// Root fields of a dimension type (`DimensionStyle`), `(field, parameter id,
/// storage)`; a bool field is an Integer 0/1 parameter.
const DIMENSION_STYLE_FIELDS: &[(&str, i64, &str)] = &[
    ("m_arrowHeadStyleId", -1006323, "ElementId"), // DIM_LEADER_ARROWHEAD
    ("m_centerlineTickMarkStyleId", -1006445, "ElementId"), // DIM_STYLE_CENTERLINE_TICK_MARK
    ("m_dimLineSnapDist", -1006446, "Double"),     // DIM_STYLE_DIM_LINE_SNAP_DIST
    ("m_dimWitnCntrlExtBelow", -1006432, "Integer"), // DIM_WITNS_LINE_CNTRL
    ("m_equalityText", -1006512, "String"),        // EQUALITY_TEXT_FOR_CONTINUOUS_LINEAR_DIM
    ("m_interiorTickMarkStyleId", -1006464, "ElementId"), // DIM_STYLE_INTERIOR_TICK_MARK
    ("m_leaderDisplayCondition", -1006510, "Integer"), // DIM_LEADER_DISPLAY_CONDITION
    ("m_leaderType", -1006508, "Integer"),         // DIM_LEADER_TYPE
    ("m_linearDimType", -1006467, "Integer"),      // LINEAR_DIM_TYPE
    ("m_showOpeningHt", -1006435, "Integer"),      // DIM_STYLE_SHOW_OPENING_HT
    ("m_textAlignment", -1006448, "Integer"),      // DIM_STYLE_READ_CONVENTION
    ("m_txtBackgroundTransparent", -1006429, "Integer"), // DIM_TEXT_BACKGROUND
];
/// Fields of a `LineAndTextAttr` (the text attributes of a dimension or text
/// type). On a dimension type the parameter sets hold a stale 0 for these:
/// the API reads the attributes, so they replace the saved slots.
const LINE_AND_TEXT_ATTR_FIELDS: &[(&str, i64, &str)] = &[
    ("m_bBold", -1006311, "Integer"),      // TEXT_STYLE_BOLD
    ("m_bItalic", -1006312, "Integer"),    // TEXT_STYLE_ITALIC
    ("m_bUnderline", -1006313, "Integer"), // TEXT_STYLE_UNDERLINE
];
/// Root fields of a text note type (`TextNoteAttributes`).
const TEXT_NOTE_TYPE_FIELDS: &[(&str, i64, &str)] = &[
    ("m_leaderOffsetSheet", -1006501, "Double"), // LEADER_OFFSET_SHEET
    ("m_textBoxVisibility", -1150213, "Integer"), // TEXT_BOX_VISIBILITY
];
/// Root fields of a view (`DBView*` records with a name).
const VIEW_FIELDS: &[(&str, i64, &str)] = &[
    ("m_fixedSketchPlaneId", -1005147, "ElementId"), // VIEW_FIXED_SKETCH_PLANE
    ("m_viewDescription", -1005114, "String"),       // VIEW_DESCRIPTION
    ("m_viewName", -1005112, "String"),              // VIEW_NAME
    ("m_viewTemplateId", -1005176, "ElementId"),     // VIEW_TEMPLATE
];
const VIEW_SHOW_HIDDEN_LINES: i64 = -1154613;
const CONCEPTUAL_CONSTRUCTION_MATERIAL: i64 = -1012024;
/// Root fields of a plan view only.
const PLAN_VIEW_FIELDS: &[(&str, i64, &str)] = &[
    ("m_bUseTrueNorth", -1005168, "Integer"), // PLAN_VIEW_NORTH
    ("m_columnSymbolElev", -1150170, "Double"), // VIEW_SLANTED_COLUMN_SYMBOL_OFFSET
    ("m_idUnderlayLevel", -1005153, "ElementId"), // VIEW_UNDERLAY_BOTTOM_ID
    ("m_idUnderlayTopLevel", -1005335, "ElementId"), // VIEW_UNDERLAY_TOP_ID
    ("m_underlayOrientation", -1005177, "Integer"), // VIEW_UNDERLAY_ORIENTATION
];
/// Views whose `ViewDisplayMgr.m_model.m_showHiddenLinesVal` is measured.
const HIDDEN_LINES_VIEW_CLASSES: &[&str] = &["DBView3d", "DBViewPlan", "DBViewSection"];
const VIEW_CLASSES: &[&str] = &[
    "DBView3d",
    "DBViewAnonDrafting",
    "DBViewDrafting",
    "DBViewLegend",
    "DBViewPlan",
    "DBViewSchedule",
    "DBViewSection",
];
/// Sub-elements whose API parameters are those of their owner
/// (`Identity.owning_element_id`): `(class, owner class, built-in ids
/// mirrored, custom parameters mirrored)`. Measured: every listed value equal
/// to the owner's on every element of the corpus.
const OWNER_MIRRORS: &[(&str, &str, &[i64], bool)] = &[
    (
        "PipeFittingCenterLine",
        "FamilyInstance",
        &[
            -1001352, // FAMILY_LEVEL_PARAM
            -1012100, // PHASE_CREATED
            -1012101, // PHASE_DEMOLISHED
            -1019000, // IFC_GUID
            -1114242, // RBS_FAMILY_CONTENT_ANNOTATION_DISPLAY
        ],
        true,
    ),
    (
        "SegmentCenterLine",
        "RbsPipeCurve",
        &[
            -1012100, // PHASE_CREATED
            -1012101, // PHASE_DEMOLISHED
            -1019000, // IFC_GUID
            -1114000, // RBS_START_LEVEL_PARAM
            -1114240, // RBS_CALCULATED_SIZE
            -1140212, // RBS_PIPE_INNER_DIAM_PARAM
            -1140225, // RBS_PIPE_DIAMETER_PARAM
            -1140238, // RBS_PIPE_OUTER_DIAMETER
            -1140334, // RBS_PIPING_SYSTEM_TYPE_PARAM
            -1150434, // RBS_REFERENCE_OVERALLSIZE
        ],
        false,
    ),
];
const OST_DIMENSIONS: i64 = -2000260;
const OST_SKETCH_DIMENSIONS: i64 = -2000261;
const OST_CONSTRAINTS: i64 = -2000262;
const OST_SPOT_ELEVATIONS: i64 = -2000263;
const OST_SPOT_SLOPES: i64 = -2000265;
const OST_LINES: i64 = -2000051;
const OST_SKETCH_LINES: i64 = -2000045;
const OST_DETAIL_COMPONENTS: i64 = -2002000;

/// Classes whose root `m_text` is the datum name (`DATUM_TEXT`).
const DATUM_TEXT_CLASSES: &[&str] = &["Grid", "Level", "RefPlane"];
/// Root field holding the family name of a type, where the file saves it.
const FAMILY_NAME_FIELDS: &[(&str, &str)] = &[
    ("FabricationPartType", "m_familyName"),
    ("MEPSystemZoneElementType", "m_systemFamilyName"),
];
/// Root fields of the HVAC load space and building types, which save their
/// settings as fields: `(field, parameter id, storage)`. Measured on every
/// element of the corpus; values already in internal units.
const HVAC_LOAD_TYPE_FIELDS: &[(&str, i64, &str)] = &[
    ("m_LightingLoadDensity", -1114220, "Double"), // ROOM_DESIGN_LIGHTING_LOAD_PER_AREA_PARAM
    ("m_PowerLoadDensity", -1114219, "Double"),    // ROOM_DESIGN_POWER_LOAD_PER_AREA_PARAM
    ("m_coolingSetPoint", -1114709, "Double"),     // SPACE_COOLING_SET_POINT
    ("m_dAirChangesPerHour", -1154671, "Double"),  // ROOM_AIR_CHANGES_PER_HOUR_PARAM
    ("m_dAreaPerPerson", -1114175, "Double"),      // ROOM_AREA_PER_PERSON_PARAM
    ("m_dInfiltration", -1114348, "Double"),       // SPACE_INFILTRATION_PARAM
    ("m_dLatentHeatGainPerPerson", -1114189, "Double"), // ROOM_PEOPLE_LATENT_HEAT_GAIN_PER_PERSON_PARAM
    ("m_dOutdoorAirPerArea", -1154668, "Double"),       // ROOM_OUTDOOR_AIR_PER_AREA_PARAM
    ("m_dOutdoorAirPerPerson", -1154665, "Double"),     // ROOM_OUTDOOR_AIR_PER_PERSON_PARAM
    ("m_dPlenumLighting", -1150188, "Double"),          // ROOM_PLENUM_LIGHTING_PARAM
    ("m_dSensibleHeatGainPerPerson", -1114239, "Double"), // ROOM_PEOPLE_SENSIBLE_HEAT_GAIN_PER_PERSON_PARAM
    ("m_dehumidificationSetPoint", -1114711, "Double"),   // SPACE_DEHUMIDIFICATION_SET_POINT
    ("m_eOutdoorAirFlowStandard", -1154687, "Integer"),   // ROOM_OUTDOOR_AIRFLOW_STANDARD_PARAM
    ("m_heatingSetPoint", -1114708, "Double"),            // SPACE_HEATING_SET_POINT
    ("m_humidificationSetPoint", -1114710, "Double"),     // SPACE_HUMIDIFICATION_SET_POINT
    ("m_idLightingSchedule", -1114350, "ElementId"),      // SPACE_LIGHTING_SCHEDULE_PARAM
    ("m_idOccupancySchedule", -1114349, "ElementId"),     // SPACE_OCCUPANCY_SCHEDULE_PARAM
    ("m_idPowerSchedule", -1114351, "ElementId"),         // SPACE_POWER_SCHEDULE_PARAM
];
/// Root fields of the HVAC load building type only.
const HVAC_LOAD_BUILDING_FIELDS: &[(&str, i64, &str)] = &[
    ("m_dCoolingSetPoint", -1114357, "Double"), // BUILDING_UNOCCUPIED_COOLING_SET_POINT_PARAM
    ("m_strEquipmentEndTime", -1114356, "String"), // BUILDING_CLOSING_TIME_PARAM
    ("m_strEquipmentStartTime", -1114355, "String"), // BUILDING_OPENING_TIME_PARAM
];

#[derive(Debug, Clone, Serialize)]
pub struct ParameterValue {
    /// `BuiltInParameter` value when negative, otherwise the id of the
    /// parameter element (project, shared or family parameter).
    pub parameter_id: i64,
    pub storage_type: String,
    pub value: Option<Value>,
    /// `parameter_set`, `saved_field`, `family_slot` or `parameter_store`
    /// (compact store of a rebar) for saved values; `owner` for a value
    /// mirrored from the owning element;
    /// `class_table` for the category of the class; `resolved` for a value
    /// read from another record (family, type); `default_verified` for the
    /// value the API reports when nothing is saved; `derived_unique_id` for
    /// an IFC identifier derived from the UniqueId.
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<&'static str>,
}

impl ParameterValue {
    fn new(parameter_id: i64, storage_type: &str, value: Value, source: &'static str) -> Self {
        Self {
            parameter_id,
            storage_type: storage_type.to_string(),
            value: Some(value),
            source,
            diagnostic: None,
        }
    }
    fn withheld(parameter_id: i64, storage_type: &str, diagnostic: &'static str) -> Self {
        Self {
            parameter_id,
            storage_type: storage_type.to_string(),
            value: None,
            source: "saved_field",
            diagnostic: Some(diagnostic),
        }
    }
}

/// One record's parameter values, keyed by parameter id, with what
/// [`resolve`] needs to complete them from other records.
#[derive(Debug)]
pub struct Projected {
    pub element_id: u64,
    pub unique_id: String,
    pub class_name: Option<String>,
    /// The record is an element type: its root carries `m_symbolInfo`.
    pub is_type: bool,
    pub values: BTreeMap<i64, ParameterValue>,
    derived_default_ifc_guid: Option<String>,
    /// `m_familyId` of a `FamilySymbol`.
    family_id: Option<i64>,
    /// `m_dbViewId` of a `Viewer`: the view whose category it reports.
    view_id: Option<i64>,
    /// The element this one belongs to (`Identity.owning_element_id`).
    owner: Option<u64>,
    /// A `CurveElem` saved as a detail curve (`m_detail`).
    detail_curve: bool,
}

/// What other records contribute to a record's values: the class of every
/// record (hosts), the family of each symbol and the category and name of
/// each family, the footing kind of each floor type.
#[derive(Debug, Default)]
pub struct Links {
    classes: HashMap<u64, String>,
    family_of_symbol: HashMap<i64, i64>,
    category_of_family: HashMap<i64, i64>,
    name_of_family: HashMap<i64, String>,
    footing_type_of_floor_type: HashMap<i64, i64>,
    /// `m_dimensionStyleType` of a `SpotElevationStyle`: 4 elevation, 8 slope.
    spot_style_kind: HashMap<i64, i64>,
}

impl Links {
    pub fn ingest(&mut self, record: &Record) {
        let Some(class) = &record.class_name else {
            return;
        };
        let id = record.identity.element_id;
        self.classes.insert(id, class.clone());
        let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return;
        };
        let Ok(id) = i64::try_from(id) else {
            return;
        };
        match class.as_str() {
            "Family" => {
                if let Some(category) = root_identifier(root, "m_categoryId") {
                    self.category_of_family.insert(id, category);
                }
                if let Some(name) = root.fields.get("m_name").and_then(Value::as_str) {
                    self.name_of_family.insert(id, name.to_string());
                }
            }
            "FamilySymbol" => {
                if let Some(family) = root_identifier(root, "m_familyId") {
                    self.family_of_symbol.insert(id, family);
                }
            }
            "FloorAttributes" => {
                if let Some(footing) = root.fields.get("m_footingType").and_then(Value::as_i64) {
                    self.footing_type_of_floor_type.insert(id, footing);
                }
            }
            "SpotElevationStyle" => {
                if let Some(kind) = root
                    .fields
                    .get("m_dimensionStyleType")
                    .and_then(Value::as_i64)
                {
                    self.spot_style_kind.insert(id, kind);
                }
            }
            _ => {}
        }
    }
}

/// The record's parameter values. Empty when the record carries no saved
/// metadata and nothing is known of its class.
pub fn project(record: &Record) -> Projected {
    let mut values = BTreeMap::new();
    let class = record.class_name.as_deref().unwrap_or("");
    if let Some(metadata) = &record.saved_metadata {
        // A family's own slots are its defaults: the API lists family
        // parameters on the symbols, never on the `Family` element.
        for p in metadata
            .resolved_family_parameters
            .iter()
            .filter(|p| p.has_value && class != "Family")
        {
            values.insert(
                p.parameter_id,
                ParameterValue::new(
                    p.parameter_id,
                    &p.storage_type,
                    p.raw_value.clone(),
                    "family_slot",
                ),
            );
        }
        for p in &metadata.raw_field_parameters {
            values.insert(
                p.parameter_id,
                ParameterValue::new(
                    p.parameter_id,
                    &p.storage_type,
                    p.raw_value.clone(),
                    "saved_field",
                ),
            );
        }
        for set in &metadata.parameter_sets {
            for p in &set.parameters {
                values.insert(
                    p.serialized_parameter_id,
                    ParameterValue::new(
                        p.serialized_parameter_id,
                        &p.storage_type,
                        p.raw_value.clone(),
                        "parameter_set",
                    ),
                );
            }
        }
    }
    let root = record.graph.as_ref().and_then(|g| g.objects.first());
    let is_type = root.is_some_and(|root| root.fields.get("m_symbolInfo").is_some());
    let mut family_id = None;
    let mut view_id = None;
    let mut detail_curve = false;
    if let (Some(graph), Some(root)) = (&record.graph, root) {
        parameter_store(class, graph, root, &mut values);
        type_id(class, graph, root, &mut values);
        if is_type {
            type_name(graph, root, &mut values);
            family_name(class, root, &mut values);
        }
        if DATUM_TEXT_CLASSES.contains(&class) {
            if let Some(text) = root.fields.get("m_text").and_then(Value::as_str) {
                values.entry(DATUM_TEXT).or_insert_with(|| {
                    ParameterValue::new(DATUM_TEXT, "String", Value::from(text), "saved_field")
                });
            }
        }
        if let Some(category) = class_category(class).or_else(|| {
            (class == "FloorAttributes")
                .then(|| root.fields.get("m_footingType").and_then(Value::as_i64))
                .flatten()
                .and_then(footing_category)
        }) {
            set_category(&mut values, category, "class_table");
        }
        // The API lists DESIGN_OPTION_ID without a value on material assets.
        if class != "PropertySetElement" {
            design_option(root, &mut values);
        }
        match class {
            "SWall" => wall_height(root, &mut values),
            "FamilySymbol" => family_id = root_identifier(root, "m_familyId"),
            "Viewer" => view_id = root_identifier(root, "m_dbViewId"),
            "HVACLoadSpaceTypeElem" => saved_fields(root, HVAC_LOAD_TYPE_FIELDS, &mut values),
            "HVACLoadBuildingTypeElem" => {
                saved_fields(root, HVAC_LOAD_TYPE_FIELDS, &mut values);
                saved_fields(root, HVAC_LOAD_BUILDING_FIELDS, &mut values);
            }
            // The default zone saves a number as its name ("0", "1") and the
            // API reports a localized "Default".
            "ZoneElement" if root.fields.get("m_bDefault") == Some(&Value::Bool(true)) => {
                if let Some(name) = values.get_mut(&ZONE_NAME) {
                    name.value = None;
                    name.diagnostic = Some("default_zone_name_localized_by_api");
                }
            }
            // The API lists these only on the user-visible dimension types
            // (the internal and unnamed ones report no settings).
            "DimensionStyle"
                if values
                    .get(&ALL_MODEL_TYPE_NAME)
                    .is_some_and(|name| name.value.is_some()) =>
            {
                saved_fields(root, DIMENSION_STYLE_FIELDS, &mut values);
                if let Some(attributes) = root
                    .fields
                    .get("m_pLineAndTextAttr")
                    .and_then(|pointer| pointer_target(graph, 0, pointer))
                    .filter(|o| o.class_name == "LineAndTextAttr")
                {
                    text_attributes(&attributes.fields, &mut values);
                }
            }
            "TextNoteAttributes" => {
                saved_fields(root, TEXT_NOTE_TYPE_FIELDS, &mut values);
                if let Some(attributes) = root.fields.get("m_lineAndTextAttr") {
                    text_attributes(attributes, &mut values);
                }
            }
            "LinearDimString" | "RadialDim" => {
                if let Some(category) = root
                    .fields
                    .get("m_flags")
                    .and_then(Value::as_i64)
                    .map(dimension_category)
                {
                    set_category(&mut values, category, "saved_field");
                }
            }
            "ConceptualConstructionType" => {
                if let Some(category) = root_identifier(root, "m_massSurfaceSubCategoryId") {
                    set_category(&mut values, category, "saved_field");
                }
                if let Some(material) = root_identifier(root, "m_materialId") {
                    values
                        .entry(CONCEPTUAL_CONSTRUCTION_MATERIAL)
                        .or_insert_with(|| {
                            ParameterValue::new(
                                CONCEPTUAL_CONSTRUCTION_MATERIAL,
                                "ElementId",
                                Value::from(material),
                                "saved_field",
                            )
                        });
                }
            }
            "FabricationPart" => {
                if let Some(category) = root_identifier(root, "m_categoryId") {
                    set_category(&mut values, category, "saved_field");
                }
            }
            "CurveElem" => {
                if unique_object(graph, "RepeatingDetailUserData").is_some() {
                    set_category(&mut values, OST_DETAIL_COMPONENTS, "saved_field");
                }
                detail_curve = root.fields.get("m_detail") == Some(&Value::Bool(true));
            }
            _ => {}
        }
        if VIEW_CLASSES.contains(&class) {
            // A view template's name and template settings are not listed by
            // the API; its hidden-line mode is.
            if root.fields.get("m_isTemplate") != Some(&Value::Bool(true)) {
                saved_fields(root, VIEW_FIELDS, &mut values);
                if class == "DBViewPlan" {
                    saved_fields(root, PLAN_VIEW_FIELDS, &mut values);
                }
            }
            if HIDDEN_LINES_VIEW_CLASSES.contains(&class) {
                if let Some(mode) = unique_object(graph, "ViewDisplayMgr")
                    .and_then(|o| o.fields.get("m_model"))
                    .and_then(|model| model.get("m_showHiddenLinesVal"))
                    .and_then(Value::as_i64)
                {
                    values.entry(VIEW_SHOW_HIDDEN_LINES).or_insert_with(|| {
                        ParameterValue::new(
                            VIEW_SHOW_HIDDEN_LINES,
                            "Integer",
                            Value::from(mode),
                            "saved_field",
                        )
                    });
                }
            }
        }
    }
    for v in values.values_mut() {
        withhold_derived(v);
    }
    Projected {
        element_id: record.identity.element_id,
        unique_id: record.identity.unique_id.clone(),
        class_name: record.class_name.clone(),
        is_type,
        values,
        derived_default_ifc_guid: record.derived_default_ifc_guid.clone(),
        family_id,
        view_id,
        owner: u64::try_from(record.identity.owning_element_id).ok(),
        detail_curve,
    }
}

/// The text attributes of a dimension or text type, replacing the saved
/// slots (stale on dimension types).
fn text_attributes(attributes: &Value, values: &mut BTreeMap<i64, ParameterValue>) {
    for &(field, parameter_id, storage) in LINE_AND_TEXT_ATTR_FIELDS {
        if let Some(flag) = attributes.get(field).and_then(Value::as_bool) {
            values.insert(
                parameter_id,
                ParameterValue::new(
                    parameter_id,
                    storage,
                    Value::from(i64::from(flag)),
                    "saved_field",
                ),
            );
        }
    }
}

/// A dimension's category from its `m_flags`: bit 4 marks a sketch
/// dimension, bit 6 a constraint (measured on 1.038 dimensions of the
/// corpus: 12 and 140 dimensions, 28 sketch, 76 constraints).
fn dimension_category(flags: i64) -> i64 {
    if flags & 0x40 != 0 {
        OST_CONSTRAINTS
    } else if flags & 0x10 != 0 {
        OST_SKETCH_DIMENSIONS
    } else {
        OST_DIMENSIONS
    }
}

/// Root fields that are the saved value of a parameter, as `(field,
/// parameter id, storage)`. A field of another shape is not projected.
fn saved_fields(
    root: &GraphObject,
    fields: &[(&str, i64, &str)],
    values: &mut BTreeMap<i64, ParameterValue>,
) {
    for &(field, parameter_id, storage) in fields {
        let Some(raw) = root.fields.get(field) else {
            continue;
        };
        let value = match storage {
            "ElementId" => identifier(raw).ok().map(Value::from),
            "Double" => raw.is_number().then(|| raw.clone()),
            "Integer" => raw
                .as_i64()
                .or_else(|| raw.as_bool().map(i64::from))
                .map(Value::from),
            "String" => raw.is_string().then(|| raw.clone()),
            _ => None,
        };
        if let Some(value) = value {
            values.entry(parameter_id).or_insert_with(|| {
                ParameterValue::new(parameter_id, storage, value, "saved_field")
            });
        }
    }
}

fn root_identifier(root: &GraphObject, field: &str) -> Option<i64> {
    root.fields.get(field).and_then(|v| identifier(v).ok())
}

/// The object a non-null pointer field of `source` points to.
fn pointer_target<'a>(
    graph: &'a ObjectGraph,
    source: usize,
    pointer: &Value,
) -> Option<&'a GraphObject> {
    if pointer.get("pointer_token")?.as_u64()? == 0 {
        return None;
    }
    let offset = pointer.get("offset")?.as_u64()? as usize;
    let edge = graph
        .edges
        .iter()
        .find(|e| e.source_object_index == source && e.pointer_offset == offset)?;
    graph.objects.get(edge.target_object_index)
}

/// The one object of `class` in the graph; none when absent or ambiguous.
fn unique_object<'a>(graph: &'a ObjectGraph, class: &str) -> Option<&'a GraphObject> {
    let mut objects = graph.objects.iter().filter(|o| o.class_name == class);
    let first = objects.next()?;
    objects.next().is_none().then_some(first)
}

/// The values of a compact parameter store ([`PARAMETER_STORES`]). Storage
/// 2 indexes the doubles, 1 the integers (both measured against the API);
/// an entry of another storage is not projected.
fn parameter_store(
    class: &str,
    graph: &ObjectGraph,
    root: &GraphObject,
    values: &mut BTreeMap<i64, ParameterValue>,
) {
    for &(_, object_class, field) in PARAMETER_STORES.iter().filter(|(c, _, _)| *c == class) {
        let object = if object_class == class {
            Some(root)
        } else {
            unique_object(graph, object_class)
        };
        let Some(store) = object.and_then(|o| o.fields.get(field)) else {
            continue;
        };
        let Some(entries) = store["m_map"]["m_map"].as_array() else {
            continue;
        };
        let arrays = &store["m_values"];
        for entry in entries {
            let Ok(id) = identifier(&entry["first"]) else {
                continue;
            };
            let (storage, array) = match entry["second"]["m_ste"].as_i64() {
                Some(2) => ("Double", "m_doubles"),
                Some(1) => ("Integer", "m_ints"),
                _ => continue,
            };
            let Some(value) = entry["second"]["m_index"]
                .as_u64()
                .and_then(|index| arrays[array].get(index as usize))
            else {
                continue;
            };
            values.entry(id).or_insert_with(|| {
                if id < 0 {
                    // The few built-in entries measured (spiral settings of
                    // a rebar) include one the API derives from the bar's
                    // geometry: not verified, withheld.
                    let mut v =
                        ParameterValue::withheld(id, storage, "rebar_store_builtin_not_verified");
                    v.source = "parameter_store";
                    v
                } else {
                    ParameterValue::new(id, storage, value.clone(), "parameter_store")
                }
            });
        }
    }
}

/// The instance's type id, in the four parameters that report it. `-1`
/// (no type, e.g. a view template) reports nothing.
fn type_id(
    class: &str,
    graph: &ObjectGraph,
    root: &GraphObject,
    values: &mut BTreeMap<i64, ParameterValue>,
) {
    let id = if let Some((_, field)) = TYPE_ID_FIELDS.iter().find(|(c, _)| *c == class) {
        root_identifier(root, field)
    } else if let Some((_, object, field)) = TYPE_ID_OBJECTS.iter().find(|(c, _, _)| *c == class) {
        unique_object(graph, object).and_then(|o| root_identifier(o, field))
    } else {
        None
    };
    let Some(id) = id.filter(|id| *id != -1) else {
        return;
    };
    for parameter_id in TYPE_ID_PARAMS {
        values.entry(parameter_id).or_insert_with(|| {
            ParameterValue::new(parameter_id, "ElementId", Value::from(id), "saved_field")
        });
    }
}

/// A type's name is `SymbolInfo.m_name`. The API hides the internal names
/// (`SecretInternal...`) and localizes the empty ones at runtime: withheld.
fn type_name(graph: &ObjectGraph, root: &GraphObject, values: &mut BTreeMap<i64, ParameterValue>) {
    let Some(name) = root
        .fields
        .get("m_symbolInfo")
        .and_then(|pointer| pointer_target(graph, 0, pointer))
        .filter(|info| info.class_name == "SymbolInfo")
        .and_then(|info| info.fields.get("m_name"))
        .and_then(Value::as_str)
    else {
        return;
    };
    values.entry(ALL_MODEL_TYPE_NAME).or_insert_with(|| {
        if name.is_empty() {
            ParameterValue::withheld(
                ALL_MODEL_TYPE_NAME,
                "String",
                "empty_saved_name_localized_by_api",
            )
        } else if name.starts_with("SecretInternal") {
            ParameterValue::withheld(
                ALL_MODEL_TYPE_NAME,
                "String",
                "internal_type_name_hidden_by_api",
            )
        } else {
            ParameterValue::new(
                ALL_MODEL_TYPE_NAME,
                "String",
                Value::from(name),
                "saved_field",
            )
        }
    });
}

/// A type's family name: saved for a few classes, read from the `Family`
/// record for a `FamilySymbol` ([`resolve`]), localized at runtime and
/// absent from the file for every system family (withheld).
fn family_name(class: &str, root: &GraphObject, values: &mut BTreeMap<i64, ParameterValue>) {
    if class == "FamilySymbol" || values.contains_key(&ALL_MODEL_FAMILY_NAME) {
        return;
    }
    let saved = FAMILY_NAME_FIELDS
        .iter()
        .find(|(c, _)| *c == class)
        .and_then(|(_, field)| root.fields.get(*field))
        .and_then(Value::as_str);
    let value = match saved {
        Some(name) => ParameterValue::new(
            ALL_MODEL_FAMILY_NAME,
            "String",
            Value::from(name),
            "saved_field",
        ),
        None => ParameterValue::withheld(
            ALL_MODEL_FAMILY_NAME,
            "String",
            "system_family_name_localized_not_in_file",
        ),
    };
    values.insert(ALL_MODEL_FAMILY_NAME, value);
}

/// A floor type's category follows its footing kind.
fn footing_category(footing_type: i64) -> Option<i64> {
    match footing_type {
        0 => Some(OST_FLOORS),
        1 => Some(OST_STRUCTURAL_FOUNDATION),
        _ => None,
    }
}

fn set_category(values: &mut BTreeMap<i64, ParameterValue>, category: i64, source: &'static str) {
    for parameter_id in [ELEM_CATEGORY_PARAM, ELEM_CATEGORY_PARAM_MT] {
        values.entry(parameter_id).or_insert_with(|| {
            ParameterValue::new(parameter_id, "ElementId", Value::from(category), source)
        });
    }
}

/// `m_designOptionId` is -1 (main model) or the design option. The sentinels
/// -4 and -3 are saved on elements the API reports in the main model (-1):
/// every element of the corpus, none of whose files has design options.
fn design_option(root: &GraphObject, values: &mut BTreeMap<i64, ParameterValue>) {
    let Some(id) = root_identifier(root, "m_designOptionId") else {
        return;
    };
    let value = match id {
        -4 | -3 => ParameterValue {
            parameter_id: DESIGN_OPTION_ID,
            storage_type: "ElementId".to_string(),
            value: Some(Value::from(-1)),
            source: "default_verified",
            diagnostic: Some("design_option_sentinel_reported_as_main_model"),
        },
        _ => ParameterValue::new(
            DESIGN_OPTION_ID,
            "ElementId",
            Value::from(id),
            "saved_field",
        ),
    };
    values.insert(DESIGN_OPTION_ID, value);
}

fn withhold_derived(v: &mut ParameterValue) {
    let diagnostic = match v.parameter_id {
        // The API computes unit weight from the structural density; the saved
        // slot is often a stale default (e.g. 7.151...).
        PHY_MATERIAL_PARAM_UNIT_WEIGHT => "derived_from_density_not_evaluated",
        // Looked up in the assembly code table, not in the element.
        ASSEMBLY_DESCRIPTION => "derived_from_assembly_code_table_not_evaluated",
        // The saved slot disagrees with the API in (almost) every measured
        // element: always 1 for the pen, mostly 1 for the leader where the API
        // reports 0. Where the API value lives is not decoded yet.
        HEAVY_END_PEN | SPOT_DIM_LEADER => "saved_slot_is_not_the_api_value",
        _ => return,
    };
    v.value = None;
    v.diagnostic = Some(diagnostic);
}

/// The wall's top constraint is the root field `m_upToLevelId` (-1: none).
/// With a top constraint the API computes the unconnected height from the
/// levels, so the saved height is withheld.
fn wall_height(root: &GraphObject, values: &mut BTreeMap<i64, ParameterValue>) {
    let up_to = root_identifier(root, "m_upToLevelId");
    if let Some(id) = up_to {
        values.insert(
            WALL_HEIGHT_TYPE,
            ParameterValue::new(
                WALL_HEIGHT_TYPE,
                "ElementId",
                Value::from(id),
                "saved_field",
            ),
        );
    }
    if let Some(height) = values.get_mut(&WALL_USER_HEIGHT_PARAM) {
        let diagnostic = match up_to {
            Some(-1) => return,
            Some(_) => "computed_from_top_constraint_not_evaluated",
            None => "top_constraint_unknown",
        };
        height.value = None;
        height.diagnostic = Some(diagnostic);
    }
}

/// Second pass over a record's values once every record is known.
///
/// - A family instance saved with a `SpanDirectionSymbol` as host reports no
///   host (-1) in the API, so that host id is withheld; so is a host whose
///   class is unknown (e.g. not among the extracted records).
/// - A family symbol takes its category and family name from its `Family`;
///   a family instance its category from its symbol's family; a floor its
///   category from its type's footing kind.
/// - An element with a category reports the IFC export flag `0` and the
///   IFC identifier derived from its UniqueId when nothing is saved.
pub fn resolve(p: &mut Projected, links: &Links) {
    let class = p.class_name.as_deref().unwrap_or("");
    if let Some(host) = p.values.get_mut(&HOST_ID_PARAM) {
        if let Some(id) = host
            .value
            .as_ref()
            .and_then(Value::as_i64)
            .filter(|id| *id != -1)
        {
            let diagnostic = match u64::try_from(id)
                .ok()
                .and_then(|id| links.classes.get(&id))
                .map(String::as_str)
            {
                Some("SpanDirectionSymbol") => Some("host_is_span_direction_symbol"),
                Some(_) => None,
                None => Some("host_class_unknown"),
            };
            if let Some(diagnostic) = diagnostic {
                host.value = None;
                host.diagnostic = Some(diagnostic);
            }
        }
    }
    let type_id = p
        .values
        .get(&SYMBOL_ID_PARAM)
        .and_then(|v| v.value.as_ref())
        .and_then(Value::as_i64);
    // A family symbol's family, or the family of the symbol an instance,
    // tag or other placed element references as its type.
    let family = match class {
        "FamilySymbol" => p.family_id,
        _ => type_id.and_then(|symbol| links.family_of_symbol.get(&symbol).copied()),
    };
    if let Some(family) = family {
        if let Some(category) = links.category_of_family.get(&family) {
            set_category(&mut p.values, *category, "resolved");
        }
        if class == "FamilySymbol" {
            if let Some(name) = links.name_of_family.get(&family) {
                p.values.entry(ALL_MODEL_FAMILY_NAME).or_insert_with(|| {
                    ParameterValue::new(
                        ALL_MODEL_FAMILY_NAME,
                        "String",
                        Value::from(name.as_str()),
                        "resolved",
                    )
                });
            }
        }
    }
    if class == "Floor" {
        if let Some(category) = type_id
            .and_then(|t| links.footing_type_of_floor_type.get(&t))
            .and_then(|footing| footing_category(*footing))
        {
            set_category(&mut p.values, category, "resolved");
        }
    }
    if class == "SpotElevation" {
        if let Some(category) = type_id
            .and_then(|style| links.spot_style_kind.get(&style))
            .and_then(|kind| match kind {
                4 => Some(OST_SPOT_ELEVATIONS),
                8 => Some(OST_SPOT_SLOPES),
                _ => None,
            })
        {
            set_category(&mut p.values, category, "resolved");
        }
    }
    // A curve in a sketch is a sketch line; a detail curve outside one is a
    // line. Model lines and area boundaries are not told apart yet.
    if class == "CurveElem" {
        let owner_class = p
            .owner
            .and_then(|owner| links.classes.get(&owner))
            .map(String::as_str);
        match (owner_class, p.detail_curve) {
            (Some("VarSketch"), _) => set_category(&mut p.values, OST_SKETCH_LINES, "resolved"),
            (None, true) => set_category(&mut p.values, OST_LINES, "resolved"),
            _ => {}
        }
    }
    // A viewer (section mark, elevation arrow, camera) reports its view's category.
    if let Some(category) = p
        .view_id
        .and_then(|view| u64::try_from(view).ok())
        .and_then(|view| links.classes.get(&view))
        .and_then(|view_class| class_category(view_class))
    {
        set_category(&mut p.values, category, "resolved");
    }
    ifc_export_defaults(p, family.is_some());
}

/// The IFC export parameters the API reports when nothing is saved: flag `0`
/// and the identifier derived from the UniqueId. Only for the elements that
/// expose them (`native_class_categories`); the identifier of a centerline
/// sub-element is derived from its owner, not decoded.
fn ifc_export_defaults(p: &mut Projected, family_based: bool) {
    let class = p.class_name.as_deref().unwrap_or("");
    let Some(category) = p
        .values
        .get(&ELEM_CATEGORY_PARAM)
        .and_then(|v| v.value.as_ref())
        .and_then(Value::as_i64)
        .filter(|c| *c != -1)
    else {
        return;
    };
    let exposes = if family_based {
        family_category_exposes_ifc_export(category)
    } else {
        class_exposes_ifc_export(class)
    };
    if !exposes {
        return;
    }
    let (flag, guid) = if p.is_type {
        (IFC_EXPORT_ELEMENT_TYPE, IFC_TYPE_GUID)
    } else {
        (IFC_EXPORT_ELEMENT, IFC_GUID)
    };
    p.values.entry(flag).or_insert_with(|| {
        ParameterValue::new(flag, "Integer", Value::from(0), "default_verified")
    });
    if class.to_ascii_lowercase().ends_with("centerline") {
        p.values.entry(guid).or_insert_with(|| {
            ParameterValue::withheld(
                guid,
                "String",
                "subelement_guid_derived_from_owner_not_decoded",
            )
        });
    } else if let Some(derived) = &p.derived_default_ifc_guid {
        p.values.entry(guid).or_insert_with(|| {
            ParameterValue::new(
                guid,
                "String",
                Value::from(derived.as_str()),
                "derived_unique_id",
            )
        });
    }
}

/// Third pass: a sub-element listed in [`OWNER_MIRRORS`] reports its owner's
/// values for the parameters the API mirrors, once every record is resolved.
/// A value the sub-element already has (other than a withheld one) stays.
pub fn mirror_owners(projected: &mut [Projected]) {
    let index: HashMap<u64, usize> = projected
        .iter()
        .enumerate()
        .map(|(i, p)| (p.element_id, i))
        .collect();
    for i in 0..projected.len() {
        let class = projected[i].class_name.as_deref().unwrap_or("");
        let Some(&(_, owner_class, builtins, custom)) =
            OWNER_MIRRORS.iter().find(|(c, _, _, _)| *c == class)
        else {
            continue;
        };
        let Some(owner) = projected[i]
            .owner
            .and_then(|owner| index.get(&owner).copied())
        else {
            continue;
        };
        if projected[owner].class_name.as_deref() != Some(owner_class) {
            continue;
        }
        let mirrored: Vec<ParameterValue> = projected[owner]
            .values
            .values()
            .filter(|v| v.value.is_some())
            .filter(|v| (custom && v.parameter_id > 0) || builtins.contains(&v.parameter_id))
            .map(|v| ParameterValue {
                parameter_id: v.parameter_id,
                storage_type: v.storage_type.clone(),
                value: v.value.clone(),
                source: "owner",
                diagnostic: None,
            })
            .collect();
        let values = &mut projected[i].values;
        for v in mirrored {
            match values.get(&v.parameter_id) {
                Some(existing) if existing.value.is_some() => {}
                _ => {
                    values.insert(v.parameter_id, v);
                }
            }
        }
    }
}
