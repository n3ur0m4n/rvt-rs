//! `rvt-gltf` — convert a Revit file to a glTF 2.0 binary (.glb).
//!
//! VW1-16 CLI wrapping the VW1-04 exporter. Reads `src`, runs the
//! `RvtDocExporter` to produce an `IfcModel` (project metadata +
//! whatever per-element info the walker can resolve), then emits
//! the glb via `ifc::gltf::model_to_glb`.
//!
//! Usage:
//!
//! ```bash
//! rvt-gltf model.rvt                    # writes model.glb next to the input
//! rvt-gltf model.rvt -o out/model.glb --verbose
//! rvt-gltf --src model.rvt --dst model.glb   # older spelling, still accepted
//! ```

use clap::Parser;
use rvt::RevitFile;
use rvt::ifc::{Exporter, RvtDocExporter, gltf::model_to_glb};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-gltf",
    version,
    about = "Convert a Revit (.rvt, .rfa, .rte, .rft) file to a glTF 2.0 binary (.glb)",
    after_help = "Examples:\n  \
        rvt-gltf model.rvt                      writes model.glb next to the input\n  \
        rvt-gltf model.rvt -o viewer/model.glb --verbose"
)]
struct Cli {
    /// Revit file to convert (.rvt / .rfa / .rte / .rft).
    #[arg(
        value_name = "INPUT",
        required_unless_present = "src",
        conflicts_with = "src"
    )]
    input: Option<PathBuf>,

    /// Output .glb path. Default: the input path with a `.glb` extension.
    #[arg(short = 'o', long = "output", alias = "dst", short_alias = 'd')]
    output: Option<PathBuf>,

    /// Older spelling of INPUT (`--src model.rvt`), kept for scripts.
    #[arg(short = 's', long = "src", hide = true)]
    src: Option<PathBuf>,

    /// Print element count + output size on success.
    #[arg(long)]
    verbose: bool,

    /// Draw doors, windows, furniture and other elements whose body would
    /// be their record's bounding box from the meshes Revit saved for
    /// display (#255). Slower on large files.
    #[arg(long)]
    saved_meshes: bool,

    /// Draw every element from its saved display mesh where it has one,
    /// adding elements rvt-rs does not export; rvt-rs bodies only where
    /// there is no saved mesh. Implies --saved-meshes.
    #[arg(long)]
    saved_first: bool,

    /// Print `progress N` lines (percent done) to stderr as the conversion
    /// goes, for front ends that show a progress bar.
    #[arg(long)]
    progress: bool,
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Input from INPUT or the legacy `--src`; output from `-o` or the input
/// path with `extension` swapped in. Refuses to overwrite the input.
fn resolve_paths(
    input: Option<&Path>,
    src: Option<&Path>,
    output: Option<&Path>,
    extension: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let input = input.or(src).ok_or("no input file given")?.to_path_buf();
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension(extension));
    if output == input {
        return Err(format!(
            "refusing to overwrite the input {}; pass -o <path>",
            input.display()
        ));
    }
    Ok((input, output))
}

fn run(cli: &Cli) -> Result<(), String> {
    let (input, output) = resolve_paths(
        cli.input.as_deref(),
        cli.src.as_deref(),
        cli.output.as_deref(),
        "glb",
    )?;
    let progress = Progress::new(cli.progress);
    // Open errors from the library already name the file.
    let mut rf = RevitFile::open(&input).map_err(|e| e.to_string())?;
    // The saved meshes do not depend on the export: read them from a second
    // handle meanwhile.
    let scene = cli.saved_first.then(|| {
        let input = input.clone();
        std::thread::spawn(move || -> anyhow::Result<_> {
            let mut rf = RevitFile::open(&input)?;
            rvt::ifc::saved_meshes::saved_scene(&mut rf)
        })
    });
    let mut model = RvtDocExporter
        .export(&mut rf)
        .map_err(|e| format!("export {}: {e}", input.display()))?;
    progress.stage("export", 1, 1);
    if !cli.saved_first {
        for (stage, _) in &STAGES[..3] {
            progress.stage(stage, 1, 1);
        }
    }
    if let Some(scene) = scene {
        let attached = scene
            .join()
            .map_err(|_| "saved graphics reader panicked".to_string())?
            .and_then(|scene| rvt::ifc::saved_meshes::attach_scene(&mut rf, &mut model, scene));
        match attached {
            Ok(report) if cli.verbose => println!(
                "rvt-gltf: saved meshes: {} drawable, {} replaced, {} added, {} ambiguous",
                report.drawable, report.replaced, report.added, report.ambiguous
            ),
            Ok(_) => {}
            Err(e) => eprintln!("rvt-gltf: saved graphics not read, bodies unchanged: {e}"),
        }
    } else if cli.saved_meshes {
        if let Err(e) = rvt::ifc::saved_meshes::attach(&mut rf, &mut model) {
            eprintln!("rvt-gltf: saved graphics not read, bodies unchanged: {e}");
        }
    }
    progress.stage("attach", 1, 1);
    let glb = model_to_glb(&model);
    fs::write(&output, &glb).map_err(|e| format!("write {}: {e}", output.display()))?;
    progress.stage("write", 1, 1);
    if cli.verbose {
        println!(
            "rvt-gltf: wrote {} bytes ({} building elements) to {}",
            glb.len(),
            model
                .entities
                .iter()
                .filter(|e| matches!(e, rvt::ifc::entities::IfcEntity::BuildingElement { .. }))
                .count(),
            output.display()
        );
    }
    Ok(())
}

/// Overall percent from the stages' own progress. A stage's share is its
/// typical part of the conversion time on large models (2026-10-21); within a
/// stage the percent follows its counted work. The export runs alongside the
/// saved-mesh stages and only holds the end when it is slower.
#[derive(Clone)]
struct Progress {
    /// Last percent printed and each stage's done fraction.
    shown: Option<std::sync::Arc<std::sync::Mutex<Shown>>>,
}

type Shown = (usize, [f64; STAGES.len()]);

const STAGES: [(&str, f64); 6] = [
    ("saved graphics", 25.),
    ("saved materials", 15.),
    ("saved meshes", 45.),
    ("export", 5.),
    ("attach", 8.),
    ("write", 2.),
];

impl Progress {
    fn new(enabled: bool) -> Self {
        let progress = Self {
            shown: enabled.then(Default::default),
        };
        if enabled {
            let sink = progress.clone();
            rvt::progress::set_sink(move |stage, done, total| sink.stage(stage, done, total));
            eprintln!("progress 0");
        }
        progress
    }

    fn stage(&self, stage: &str, done: usize, total: usize) {
        let Some(shown) = &self.shown else { return };
        let Some(k) = STAGES.iter().position(|(name, _)| *name == stage) else {
            return;
        };
        let mut shown = shown.lock().unwrap_or_else(|e| e.into_inner());
        shown.1[k] = shown.1[k].max(done as f64 / total.max(1) as f64);
        let percent = STAGES
            .iter()
            .zip(shown.1)
            .map(|((_, share), fraction)| share * fraction)
            .sum::<f64>()
            .floor() as usize;
        if percent > shown.0 {
            shown.0 = percent;
            eprintln!("progress {percent}");
        }
    }
}
