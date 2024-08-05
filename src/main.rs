#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]

use std::time::Instant;

pub mod types;
pub mod utils;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const BENCHMARKS_FILE_PATH: &str = "./benchmarks.json";

fn main() -> Result<()> {
    // Get the program-specific epoch
    let epoch = Instant::now();

    // Get the system specifications
    let system_specs = utils::system_specs()?;

    // Create a mutable array of new benchmarks to be performed
    let mut benchmarks = utils::generate_benchmarks("./tests/")?;

    // Get the start time of the entire benchmarking process
    let start_time = std::time::Instant::now();

    // Run all of the benchmarks
    for benchmark in &mut benchmarks {
        benchmark.run(&epoch);
    }

    // Get the end time of the entire benchmarking process
    let end_time = std::time::Instant::now();

    // Print the benchmark results
    print_benchmarks(start_time, end_time, &benchmarks);

    // Store the benchmark results
    store_benchmarks(&types::Benchmarks {
        system_specs,
        benchmarks,
    })?;

    Ok(())
}

/// Store the benchmark results in a file
fn store_benchmarks(benchmarks: &types::Benchmarks) -> Result<()> {
    let benchmarks_json_string = serde_json::to_string_pretty(&benchmarks)?;
    std::fs::write(BENCHMARKS_FILE_PATH, benchmarks_json_string)?;
    Ok(())
}

/// Print the benchmark results
/// This is only used only for debugging purposes
fn print_benchmarks(start_time: Instant, end_time: Instant, benchmarks: &Vec<types::Benchmark>) {
    // Display the benchmark results
    println!(
        "Benchmarking took {:?} in total:",
        end_time.duration_since(start_time),
    );

    for benchmark in benchmarks {
        println!(
            "    Benchmark \"{}\" took {:?} in total:",
            benchmark.name,
            benchmark.end_time.unwrap(),
        );

        for phase in &benchmark.phases {
            println!(
                "        Phase \"{}\" took {:?} in total:",
                phase.name,
                phase.end_time.unwrap(),
            );
        }
        println!(
            "            {}",
            format!("{:#?}", benchmark.frames).replace('\n', "\n            "),
        );
    }
}
