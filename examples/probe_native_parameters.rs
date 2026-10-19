//! Probe: what the native extraction (`native_document::extract`) decodes as
//! parameters, element by element.
//!
//! Writes one JSON line per record: element id, class, status, diagnostic and
//! the saved metadata (parameter sets, raw field parameters, declared custom
//! parameters). The last line is the summary's parameter definitions and unit
//! formats. Values stay serialized: nothing is evaluated or converted.
//!
//! With `--graph` each line also carries the element's decoded object graph.
//!
//! Usage: probe_native_parameters [--graph] <model.rvt> [element id ...]

use rvt::{RevitFile, native_document};
use serde_json::json;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).peekable();
    let graph = args.next_if_eq("--graph").is_some();
    let path = PathBuf::from(args.next().expect("model path"));
    let mut options = native_document::Options::default();
    for id in args {
        options.selected_ids.insert(id.parse()?);
    }
    let mut rf = RevitFile::open(&path)?;
    let summary = native_document::extract(&mut rf, &options, |record| {
        let mut line = json!({
            "id": record.identity.element_id,
            "unique_id": record.identity.unique_id,
            "class": record.class_name,
            "status": record.status,
            "diagnostic": record.diagnostic,
            "metadata_diagnostic": record.metadata_diagnostic,
            "saved_metadata": record.saved_metadata,
        });
        if graph {
            line["graph"] = json!(record.graph);
        }
        println!("{line}");
        Ok(())
    })?;
    println!(
        "{}",
        json!({
            "summary": {
                "revit_version": summary.revit_version,
                "indexed_elements": summary.indexed_elements,
                "emitted_records": summary.emitted_records,
                "complete_graph_records": summary.complete_graph_records,
                "unsupported_graph_records": summary.unsupported_graph_records,
                "projected_metadata_records": summary.projected_metadata_records,
                "unsupported_metadata_records": summary.unsupported_metadata_records,
                "parameter_definitions": summary.parameter_definitions,
                "definition_diagnostics": summary.definition_diagnostics,
                "unit_formats": summary.unit_formats,
                "parameter_binding_context": summary.parameter_binding_context,
            }
        })
    );
    Ok(())
}
