//! `rvt-views` — the views and sheets of a model as data: names, types,
//! scales, levels, templates, the views placed on each sheet and the title
//! blocks. Nothing is drawn.
//!
//! Read from the native extraction (`native_parameter_values::extract_values`)
//! and the views' root fields, measured against API exports of the same files.

use clap::Parser;
use rvt::RevitFile;
use rvt::native_document::{self, Record};
use rvt::native_parameter_values::{self, Projected};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-views",
    version,
    about = "List the views and sheets of a model as JSON",
    after_help = "Example:\n  rvt-views model.rvt > views.json"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    file: PathBuf,
}

const ELEM_CATEGORY_PARAM: i64 = -1140362;
const ALL_MODEL_TYPE_NAME: i64 = -1002001;
const OST_TITLE_BLOCKS: i64 = -2000280;
const OST_SHEETS: i64 = -2003100;
const ELEM_TYPE_PARAM: i64 = -1002050;

#[derive(Serialize)]
struct Out {
    revit_version: u32,
    views: Vec<ViewOut>,
    sheets: Vec<SheetOut>,
}

#[derive(Serialize)]
struct ViewOut {
    id: u64,
    unique_id: String,
    class_name: String,
    name: Option<String>,
    is_template: bool,
    /// The view family type.
    type_id: Option<i64>,
    type_name: Option<String>,
    /// `1 : scale`, from the saved scale factor.
    scale: Option<i64>,
    /// The level a plan is generated from.
    level_id: Option<i64>,
    template_id: Option<i64>,
    /// The sheet the view is placed on, through a viewport.
    sheet_id: Option<u64>,
}

#[derive(Serialize)]
struct SheetOut {
    id: u64,
    unique_id: String,
    number: Option<String>,
    name: Option<String>,
    type_id: Option<i64>,
    viewports: Vec<ViewportOut>,
    title_blocks: Vec<TitleBlockOut>,
}

#[derive(Serialize)]
struct ViewportOut {
    id: u64,
    view_id: Option<i64>,
    detail_number: Option<String>,
    type_id: Option<i64>,
}

#[derive(Serialize)]
struct TitleBlockOut {
    id: u64,
    type_id: Option<i64>,
}

/// The root fields of a view, sheet, viewport or title block.
struct Root {
    class: String,
    fields: serde_json::Map<String, Value>,
}

impl Root {
    fn id(&self, field: &str) -> Option<i64> {
        self.fields
            .get(field)?
            .get("m_id")?
            .get("m_id64")?
            .as_i64()
            .filter(|id| *id != -1)
    }
    fn text(&self, field: &str) -> Option<String> {
        self.fields.get(field)?.as_str().map(str::to_string)
    }
}

/// View classes listed. Not listed: view family types (`DBViewType*`) and
/// the internal project view (`DBViewProject`, no saved name).
fn is_view(class: &str) -> bool {
    class.starts_with("DBView") && !class.starts_with("DBViewType") && class != "DBViewProject"
}

/// A view whose `m_unplacedOwnerId` is set is not an element of the model
/// in the API (measured: none of them is listed).
fn is_unplaced(root: &Root) -> bool {
    root.id("m_unplacedOwnerId").is_some()
}

/// An elevation saves no type: it is the `m_symbolId` of its marker
/// (`InteriorElev`), the parent of the view's arrow (`InteriorElevArrow`).
fn elevation_type(view: &Root, roots: &HashMap<u64, Root>) -> Option<i64> {
    let arrow = roots.get(&u64::try_from(view.id("m_viewerId")?).ok()?)?;
    if arrow.class != "InteriorElevArrow" {
        return None;
    }
    let marker = roots.get(&u64::try_from(arrow.id("m_parentId")?).ok()?)?;
    (marker.class == "InteriorElev").then(|| marker.id("m_symbolId"))?
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
    let mut roots: HashMap<u64, Root> = HashMap::new();
    let keep = |record: &Record| {
        let class = record.class_name.as_deref()?;
        if !(class.starts_with("DBView")
            || matches!(
                class,
                "Viewport" | "FamilyInstance" | "InteriorElevArrow" | "InteriorElev"
            ))
        {
            return None;
        }
        let root = record.graph.as_ref()?.objects.first()?;
        Some(Root {
            class: class.to_string(),
            fields: root.fields.as_object()?.clone(),
        })
    };
    let (projected, summary) = native_parameter_values::extract_values(
        &mut rf,
        &native_document::Options::default(),
        |record| {
            if let Some(root) = keep(record) {
                roots.insert(record.identity.element_id, root);
            }
        },
    )?;
    let values: HashMap<u64, &Projected> = projected.iter().map(|p| (p.element_id, p)).collect();
    let value = |id: u64, parameter: i64| values.get(&id)?.values.get(&parameter)?.value.clone();
    let category = |id: u64| value(id, ELEM_CATEGORY_PARAM).and_then(|v| v.as_i64());
    let type_name = |id: Option<i64>| {
        let id = u64::try_from(id?).ok()?;
        value(id, ALL_MODEL_TYPE_NAME).and_then(|v| v.as_str().map(str::to_string))
    };

    let mut sheets: BTreeMap<u64, SheetOut> = BTreeMap::new();
    let mut placed: HashMap<i64, u64> = HashMap::new();
    let mut viewports: Vec<(u64, ViewportOut)> = Vec::new();
    let mut title_blocks: Vec<(u64, TitleBlockOut)> = Vec::new();
    for (&id, root) in &roots {
        match root.class.as_str() {
            "Viewport" => {
                let Some(sheet) = root
                    .id("m_ownerDBViewId")
                    .and_then(|s| u64::try_from(s).ok())
                else {
                    continue;
                };
                let view_id = root.id("m_dbViewId");
                if let Some(view) = view_id {
                    placed.insert(view, sheet);
                }
                viewports.push((
                    sheet,
                    ViewportOut {
                        id,
                        view_id,
                        detail_number: root.text("m_detailNumber"),
                        type_id: root.id("m_attrId"),
                    },
                ));
            }
            "FamilyInstance" if category(id) == Some(OST_TITLE_BLOCKS) => {
                if let Some(sheet) = root
                    .id("m_ownerDBViewId")
                    .and_then(|s| u64::try_from(s).ok())
                {
                    title_blocks.push((
                        sheet,
                        TitleBlockOut {
                            id,
                            type_id: value(id, ELEM_TYPE_PARAM).and_then(|v| v.as_i64()),
                        },
                    ));
                }
            }
            // A sheet record with an empty number is not a sheet in the API
            // (one case measured, number and name both empty).
            class
                if is_view(class)
                    && !is_unplaced(root)
                    && category(id) == Some(OST_SHEETS)
                    && root.text("m_sheetNumber").is_some_and(|n| !n.is_empty()) =>
            {
                sheets.insert(
                    id,
                    SheetOut {
                        id,
                        unique_id: values
                            .get(&id)
                            .map(|p| p.unique_id.clone())
                            .unwrap_or_default(),
                        number: root.text("m_sheetNumber"),
                        name: root.text("m_viewName"),
                        type_id: root.id("m_dbViewTypeId"),
                        viewports: Vec::new(),
                        title_blocks: Vec::new(),
                    },
                );
            }
            _ => {}
        }
    }
    for (sheet, vp) in viewports {
        if let Some(s) = sheets.get_mut(&sheet) {
            s.viewports.push(vp);
        }
    }
    for (sheet, tb) in title_blocks {
        if let Some(s) = sheets.get_mut(&sheet) {
            s.title_blocks.push(tb);
        }
    }
    for s in sheets.values_mut() {
        s.viewports.sort_by_key(|v| v.id);
        s.title_blocks.sort_by_key(|t| t.id);
    }

    let mut views: Vec<ViewOut> = roots
        .iter()
        .filter(|(id, root)| {
            is_view(&root.class) && !is_unplaced(root) && category(**id) != Some(OST_SHEETS)
        })
        .map(|(&id, root)| {
            // The resolved type id (also for views that save none, e.g. an
            // elevation), else the saved one.
            let type_id = value(id, ELEM_TYPE_PARAM)
                .and_then(|v| v.as_i64())
                .filter(|t| *t != -1)
                .or_else(|| root.id("m_dbViewTypeId"))
                .or_else(|| elevation_type(root, &roots));
            ViewOut {
                id,
                unique_id: values
                    .get(&id)
                    .map(|p| p.unique_id.clone())
                    .unwrap_or_default(),
                class_name: root.class.clone(),
                name: root.text("m_viewName"),
                is_template: root.fields.get("m_isTemplate") == Some(&Value::Bool(true)),
                type_id,
                type_name: type_name(type_id),
                scale: root
                    .fields
                    .get("m_scale")
                    .and_then(Value::as_f64)
                    .filter(|s| *s > 0.0)
                    .map(|s| (1.0 / s).round() as i64),
                level_id: root.id("m_genElemId"),
                template_id: root.id("m_viewTemplateId"),
                sheet_id: i64::try_from(id).ok().and_then(|v| placed.get(&v).copied()),
            }
        })
        .collect();
    views.sort_by_key(|v| v.id);

    let out = Out {
        revit_version: summary.revit_version,
        views,
        sheets: sheets.into_values().collect(),
    };
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
