//! Print the saved-graphics objects of elements as JSON lines, model feet:
//! `{"id", "index", "class", "fields"}`; with classes given, only those.
//!
//! Usage: `dump_graphics_objects <file.rvt> <id[,id...]> [class ...]`

use std::collections::BTreeSet;
use std::io::Write;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: dump_graphics_objects <file.rvt> <ids> [class ...]");
    let ids: BTreeSet<u64> = args
        .next()
        .expect("element ids")
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    let classes: BTreeSet<String> = args.collect();
    let mut rf = rvt::RevitFile::open(&path)?;
    let options = rvt::native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    rvt::native_document::extract(&mut rf, &options, |r| {
        if let Some(g) = &r.graph {
            for (index, o) in g.objects.iter().enumerate() {
                if classes.is_empty() || classes.contains(&o.class_name) {
                    serde_json::to_writer(
                        &mut out,
                        &serde_json::json!({"id": r.identity.element_id, "index": index,
                            "class": o.class_name, "fields": o.fields}),
                    )?;
                    writeln!(out)?;
                }
            }
        }
        Ok(())
    })?;
    Ok(())
}
