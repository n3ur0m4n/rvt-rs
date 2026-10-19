//! `rvt-params` — parameter values of every element, from the native
//! extraction (`native_document::extract`).
//!
//! Values are in internal units as serialized; values Revit derives at
//! runtime are withheld with a diagnostic (`native_parameter_values`).
//! Custom parameter ids resolve through `definitions`; built-in parameters
//! carry only their negative `BuiltInParameter` id.

use clap::Parser;
use rvt::native_parameter_values::{self, ParameterValue};
use rvt::{RevitFile, native_document};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-params",
    version,
    about = "Dump element parameter values from the native extraction as JSON",
    after_help = "Examples:\n  \
        rvt-params model.rvt > params.json\n  \
        rvt-params model.rvt --id 211505 --id 211506"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    file: PathBuf,

    /// Only these element ids (repeatable). Default: every element.
    #[arg(long = "id")]
    ids: Vec<u64>,
}

#[derive(Serialize)]
struct Out {
    revit_version: u32,
    elements: Vec<ElementOut>,
    definitions: Vec<DefinitionOut>,
}

#[derive(Serialize)]
struct ElementOut {
    id: u64,
    unique_id: String,
    class_name: Option<String>,
    parameters: Vec<ParameterValue>,
}

#[derive(Serialize)]
struct DefinitionOut {
    parameter_id: i64,
    name: String,
    shared_guid: Option<String>,
    storage_type: Option<String>,
    spec_type_id: Option<String>,
    group_type_id: Option<String>,
}

fn main() -> std::process::ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut rf = RevitFile::open(&cli.file)?;
    let mut options = native_document::Options::default();
    options.selected_ids.extend(cli.ids);
    let mut elements = Vec::new();
    let mut classes = HashMap::new();
    let summary = native_document::extract(&mut rf, &options, |record| {
        if let Some(class) = &record.class_name {
            classes.insert(record.identity.element_id, class.clone());
        }
        let parameters = native_parameter_values::project(&record);
        if !parameters.is_empty() {
            elements.push(ElementOut {
                id: record.identity.element_id,
                unique_id: record.identity.unique_id.clone(),
                class_name: record.class_name.clone(),
                parameters,
            });
        }
        Ok(())
    })?;
    for element in &mut elements {
        native_parameter_values::resolve_hosts(&mut element.parameters, |id| {
            u64::try_from(id)
                .ok()
                .and_then(|id| classes.get(&id).cloned())
        });
    }
    let definitions = summary
        .parameter_definitions
        .into_values()
        .map(|d| DefinitionOut {
            parameter_id: d.parameter_id,
            name: d.caption,
            shared_guid: d.shared_guid,
            storage_type: d.storage_type,
            spec_type_id: d.spec_type_id,
            group_type_id: d.group_type_id,
        })
        .collect();
    let out = Out {
        revit_version: summary.revit_version,
        elements,
        definitions,
    };
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
