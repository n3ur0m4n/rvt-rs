//! Dump every element's saved display mesh as JSON lines, model feet:
//! `{"id", "status", "vertices": [x, y, z, ...], "triangles": [a, b, c, ...]}`.
//!
//! Usage: `dump_saved_meshes <file.rvt> [detail 1-3]`

use std::io::Write;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: dump_saved_meshes <file.rvt> [detail]");
    let detail: i64 = args.next().map(|d| d.parse()).transpose()?.unwrap_or(3);
    let mut rf = rvt::RevitFile::open(&path)?;
    let options = rvt::native_document::Options {
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = rvt::native_saved_scene::extract_at_detail(&mut rf, &options, detail)?;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for element in &scene.elements {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for primitive in &element.meshes.primitives {
            let base = (vertices.len() / 3) as u32;
            let n = primitive.vertices.len() as u32;
            vertices.extend(primitive.vertices.iter().flatten().copied());
            triangles.extend(
                primitive
                    .triangles
                    .iter()
                    .filter(|t| t.iter().all(|&v| v < n))
                    .flat_map(|t| t.map(|v| base + v)),
            );
        }
        serde_json::to_writer(
            &mut out,
            &serde_json::json!({
                "id": element.identity.element_id,
                "status": element.status,
                "vertices": vertices,
                "triangles": triangles,
            }),
        )?;
        writeln!(out)?;
    }
    Ok(())
}
