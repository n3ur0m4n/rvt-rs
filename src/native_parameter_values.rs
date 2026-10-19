//! Parameter values of one native record, as the Revit API reports them.
//!
//! Sources, in order of precedence for the same parameter id: the element's
//! saved parameter sets (`m_pParamValueSet*`), the directly projected saved
//! fields, then the family parameter slots of a family type. Values stay in
//! internal units, as serialized.
//!
//! Some serialized values are not the value the API reports, because Revit
//! derives the parameter at runtime. Measured against API exports of the same
//! files (Revit 2024-2027), those values are withheld: `value` is `None` and
//! `diagnostic` says why. Nothing is evaluated here.

use crate::native_document::Record;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

const ASSEMBLY_DESCRIPTION: i64 = -1002501;
const DESIGN_OPTION_ID: i64 = -1013201;
const HEAVY_END_PEN: i64 = -1006447;
const HOST_ID_PARAM: i64 = -1002108;
const SPOT_DIM_LEADER: i64 = -1006466;
const PHY_MATERIAL_PARAM_UNIT_WEIGHT: i64 = -1140309;
const WALL_USER_HEIGHT_PARAM: i64 = -1001105;
const WALL_HEIGHT_TYPE: i64 = -1001103;

#[derive(Debug, Clone, Serialize)]
pub struct ParameterValue {
    /// `BuiltInParameter` value when negative, otherwise the id of the
    /// parameter element (project, shared or family parameter).
    pub parameter_id: i64,
    pub storage_type: String,
    pub value: Option<Value>,
    /// `parameter_set`, `saved_field` or `family_slot`.
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<&'static str>,
}

/// Values of the record's parameters, sorted by parameter id. Empty when the
/// record carries no saved metadata.
pub fn project(record: &Record) -> Vec<ParameterValue> {
    let Some(metadata) = &record.saved_metadata else {
        return Vec::new();
    };
    let mut values = BTreeMap::new();
    for p in metadata
        .resolved_family_parameters
        .iter()
        .filter(|p| p.has_value)
    {
        values.insert(
            p.parameter_id,
            value(p.parameter_id, &p.storage_type, &p.raw_value, "family_slot"),
        );
    }
    for p in &metadata.raw_field_parameters {
        values.insert(
            p.parameter_id,
            value(p.parameter_id, &p.storage_type, &p.raw_value, "saved_field"),
        );
    }
    for set in &metadata.parameter_sets {
        for p in &set.parameters {
            values.insert(
                p.serialized_parameter_id,
                value(
                    p.serialized_parameter_id,
                    &p.storage_type,
                    &p.raw_value,
                    "parameter_set",
                ),
            );
        }
    }
    if record.class_name.as_deref() == Some("SWall") {
        wall_height(record, &mut values);
    }
    for v in values.values_mut() {
        withhold_derived(v);
    }
    values.into_values().collect()
}

fn value(id: i64, storage_type: &str, raw: &Value, source: &'static str) -> ParameterValue {
    ParameterValue {
        parameter_id: id,
        storage_type: storage_type.to_string(),
        value: Some(raw.clone()),
        source,
        diagnostic: None,
    }
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
        // -1 is the main model in both the file and the API; -4 is saved where
        // the API reports -1, with no design options in the measured files.
        DESIGN_OPTION_ID if v.value.as_ref().and_then(Value::as_i64) == Some(-4) => {
            "unresolved_design_option_sentinel"
        }
        _ => return,
    };
    v.value = None;
    v.diagnostic = Some(diagnostic);
}

/// The wall's top constraint is the root field `m_upToLevelId` (-1: none).
/// With a top constraint the API computes the unconnected height from the
/// levels, so the saved height is withheld.
fn wall_height(record: &Record, values: &mut BTreeMap<i64, ParameterValue>) {
    let up_to = record
        .graph
        .as_ref()
        .and_then(|g| g.objects.first())
        .and_then(|root| root.fields.get("m_upToLevelId"))
        .and_then(|f| crate::native_metadata::identifier(f).ok());
    if let Some(id) = up_to {
        values.insert(
            WALL_HEIGHT_TYPE,
            ParameterValue {
                parameter_id: WALL_HEIGHT_TYPE,
                storage_type: "ElementId".to_string(),
                value: Some(Value::from(id)),
                source: "saved_field",
                diagnostic: None,
            },
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

/// Second pass over a record's values once every record's class is known.
/// A family instance saved with a `SpanDirectionSymbol` as host reports no
/// host (-1) in the API, so that host id is withheld; so is a host whose class
/// is unknown (e.g. not among the extracted records).
pub fn resolve_hosts(values: &mut [ParameterValue], class_of: impl Fn(i64) -> Option<String>) {
    for v in values
        .iter_mut()
        .filter(|v| v.parameter_id == HOST_ID_PARAM)
    {
        let Some(host) = v.value.as_ref().and_then(Value::as_i64) else {
            continue;
        };
        if host == -1 {
            continue;
        }
        let diagnostic = match class_of(host).as_deref() {
            Some("SpanDirectionSymbol") => "host_is_span_direction_symbol",
            Some(_) => continue,
            None => "host_class_unknown",
        };
        v.value = None;
        v.diagnostic = Some(diagnostic);
    }
}
