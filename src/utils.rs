use crate::types::{Benchmark, SystemSpecs};

/// Collect all profiling targets in the given directory and return a map of the target name to the path canonical path.
pub fn generate_benchmarks<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<Vec<Benchmark>, Box<dyn std::error::Error>> {
    let path = path.as_ref();

    let mut targets = Vec::new();
    for entry in walkdir::WalkDir::new(path)
        .max_depth(2)
        .min_depth(2)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        let entry_path = entry.path();
        let canonical_path = std::fs::canonicalize(entry_path)?;

        if let Some(name) = canonical_path.file_name().and_then(|n| n.to_str()) {
            let benchmark = Benchmark::new(&name.to_string(), canonical_path.clone());
            if benchmark.verify_path() {
                targets.push(benchmark);
            }
        }
    }

    Ok(targets)
}

/// Returns the full system specifications as a `SystemSpecs` struct.
pub fn system_specs() -> Result<crate::types::SystemSpecs, Box<dyn std::error::Error>> {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let system_specs_string = serde_json::to_string(&sys)?;
    let system_specs: SystemSpecs = serde_json::from_str(&system_specs_string)?;
    Ok(system_specs)
}
