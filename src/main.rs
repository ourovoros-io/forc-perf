#![warn(clippy::all, clippy::pedantic)]

use types::Benchmarks;

pub mod types;
pub mod utils;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    // Get the system specifications
    let system_specs = utils::system_specs()?;
    // Create a mutable array of new benchmarks to be performed
    let mut benchmarks = utils::generate_benchmarks("./tests/")?;

    // Get the start time of the entire benchmarking process
    let start_time = std::time::Instant::now();

    // Run all of the benchmarks
    for benchmark in &mut benchmarks {
        benchmark.run();
    }

    // Get the end time of the entire benchmarking process
    let end_time = std::time::Instant::now();

    // Display the benchmark results
    println!(
        "Benchmarking took {:?} in total:",
        end_time.duration_since(start_time),
    );

    for benchmark in &benchmarks {
        println!(
            "    Benchmark \"{}\" took {:?} in total:",
            benchmark.name,
            benchmark
                .end_time
                .clone()
                .unwrap()
                .duration_since(*benchmark.start_time.clone().unwrap()),
        );

        for phase in &benchmark.phases {
            println!(
                "        Phase \"{}\" took {:?} in total:",
                phase.name,
                phase
                    .end_time
                    .clone()
                    .unwrap()
                    .duration_since(*phase.start_time.clone().unwrap()),
            );
        }
        println!(
            "            {}",
            format!("{:#?}", benchmark.frames).replace('\n', "\n            "),
        );
    }
    
    let benchmarks_output = Benchmarks {
        system_specs,
        benchmarks,
    };

    let benchmarks_json_string = serde_json::to_string_pretty(&benchmarks_output)?;
    std::fs::write("./benchmarks.json", benchmarks_json_string)?;

    Ok(())
}
