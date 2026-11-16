#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! Original clean-room probes. No installed library graphs or assets are used.

#[path = "catalog_parity/atomic_cases.rs"]
mod atomic_cases;
#[path = "catalog_parity/cases.rs"]
mod cases;
#[path = "catalog_parity/comparison.rs"]
mod comparison;
#[path = "catalog_parity/control_cases.rs"]
mod control_cases;
#[path = "catalog_parity/function_cases.rs"]
mod function_cases;
#[path = "catalog_parity/measurement.rs"]
mod measurement;
#[path = "catalog_parity/pattern_cases.rs"]
mod pattern_cases;
#[path = "catalog_parity/processor_cases.rs"]
mod processor_cases;
#[path = "catalog_parity/reference_run.rs"]
mod reference_run;
#[path = "catalog_parity/supplemental_cases.rs"]
mod supplemental_cases;

#[path = "catalog_parity/probe_case.rs"]
mod probe_case;
use probe_case::ProbeCase;
use serde_json::json;
use std::{collections::BTreeSet, path::Path, sync::Mutex};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Bound native renderer processes and their per-process memory
const PROBE_WORKERS: usize = 4;
const PROGRESS_INTERVAL_CASES: usize = 25;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let directory = args.get(1).ok_or("usage: catalog_parity <results-directory> [reference-tool-directory] [case-filter]")?;
    let directory = Path::new(directory);
    std::fs::create_dir_all(directory)?;
    let inventory = json!({
        "atomic": materialcraft_nodes::catalog().iter().map(|definition| json!({
            "id": definition.id,
            "inputs": definition.inputs.iter().map(|p| json!({"id": p.id, "kind": format!("{:?}", p.kind)})).collect::<Vec<_>>(),
            "outputs": definition.outputs.iter().map(|p| json!({"id": p.id, "kind": format!("{:?}", p.kind)})).collect::<Vec<_>>(),
            "parameters": definition.params.iter().map(|p| json!({"id": p.id, "default": p.default, "min": p.min, "max": p.max})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "function": materialcraft_nodes::function::FUNCTIONS.iter().map(|definition| json!({
            "id": definition.id, "inputs": definition.inputs, "default": materialcraft_nodes::function::default_value(definition.id),
        })).collect::<Vec<_>>(),
        "pattern": materialcraft_nodes::pattern::definitions().iter().map(|definition| json!({
            "id": definition.id,
            "parameters": definition.params.iter().map(|p| json!({"id": p.id, "default": p.default})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    std::fs::write(directory.join("inventory.json"), serde_json::to_vec_pretty(&inventory)?)?;
    let library = materialcraft_engine::NodeLibrary::shared();
    let library_inventory: Vec<_> = library
        .packages
        .iter()
        .flat_map(|(uri, package)| {
            package.graphs.iter().filter(|graph| graph.function.is_some()).map(move |graph| {
                json!({
                    "package": uri, "path": graph.sbs_path(), "nodes": graph.function.as_ref().map_or(0, |f| f.nodes.len()),
                    "status": "untested_library_graph", "hidden": graph.attributes.hide_in_library,
                })
            })
        })
        .collect();
    std::fs::write(directory.join("library-inventory.json"), serde_json::to_vec_pretty(&library_inventory)?)?;
    let tools = args.get(2).map(Path::new);
    let filter = args.get(3).map(String::as_str).unwrap_or("");
    let cases: Vec<_> = cases::cases()?.into_iter().filter(|case| case.name.contains(filter)).collect();
    let mut names = BTreeSet::new();
    for case in &cases {
        if !names.insert(&case.name) {
            return Err(format!("duplicate probe name: {}", case.name).into());
        }
    }
    if cases.is_empty() {
        return Err("no probes match the filter".into());
    }

    let report = Mutex::new(Vec::<serde_json::Value>::new());
    std::thread::scope(|scope| -> Result<()> {
        let mut workers = Vec::new();
        for group in cases.chunks(cases.len().div_ceil(PROBE_WORKERS)) {
            let report = &report;
            let total = cases.len();
            workers.push(scope.spawn(move || -> std::result::Result<(), String> {
                for case in group {
                    let result = match measurement::measure(case, directory, tools) {
                        Ok(result) => result,
                        Err(error) => json!({"case": case.name, "status": "harness_error", "error": error.to_string()}),
                    };
                    let mut report = report.lock().map_err(|_| "probe report lock is poisoned")?;
                    report.push(result);
                    let bytes = serde_json::to_vec_pretty(&*report).map_err(|error| error.to_string())?;
                    std::fs::write(directory.join("report.json"), bytes).map_err(|error| error.to_string())?;
                    if report.len() % PROGRESS_INTERVAL_CASES == 0 || report.len() == total {
                        println!("measured {} / {total}", report.len());
                    }
                }
                Ok(())
            }));
        }
        for worker in workers {
            worker.join().map_err(|_| "probe worker panicked")??;
        }
        Ok(())
    })?;
    let mut report = report.into_inner().map_err(|_| "probe report lock is poisoned")?;
    report.sort_by(|a, b| a["case"].as_str().cmp(&b["case"].as_str()));
    std::fs::write(directory.join("report.json"), serde_json::to_vec_pretty(&report)?)?;

    Ok(())
}
