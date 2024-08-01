# Forc-Perf

## Description

This project is a profiler for the Sway compiler. It is designed to be lightweight and does not poison the data that it collects from the compiler.
It collects frames of data and timestamps for all the different compiler passes and the time it takes to run them. It also collects the time it takes to run the entire compiler.

## Design

```mermaid
    flowchart LR
        A[Forc-Perf] --> |spawn| B(Forc)
        A --> |spawn| C[Collector]
        B --> |start|C
        C --> |stop|B
        C --> |collect| F[Frames]
        F --> |collect|A
        B --> |run| D[Target]
```

The Forc-perf performs the following steps:

- Spawns the Forc compiler with the appropriate arguments.

- Spawns the collector.

- The compiler indicates to the collector that it is starting.

- The collector starts collecting data and timestamps for the compiler passes.

- The compiler indicates to the collector that it is stopping.

- The collector constructs the data to be returned to the Forc-perf.

## Data Structures

```rust
/// Benchmark metadata and phase-specific performance data.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Benchmark {
    /// The name of the benchmark.
    pub name: String,
    /// The path to the benchmark's project folder.
    pub path: PathBuf,
    /// The start time of the benchmark.
    pub start_time: Option<SerdeInstant>,
    /// The end time of the benchmark.
    pub end_time: Option<SerdeInstant>,
    /// The phases of the benchmark.
    pub phases: Vec<BenchmarkPhase>,
    /// The performance frames collected from the benchmark.
    pub frames: SerdeFrames,
}

/// A named collection of performance frames representing a single phase of a benchmark.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BenchmarkPhase {
    /// The name of the benchmark phase.
    pub name: String,
    /// The start time of the benchmark phase.
    pub start_time: Option<SerdeInstant>,
    /// The end time of the benchmark phase.
    pub end_time: Option<SerdeInstant>,
}

/// A single frame of performance information for a benchmark phase.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BenchmarkFrame {
    /// The time that the frame was captured.
    pub timestamp: SerdeInstant,
    /// The process-specific CPU usage at the time the frame was captured.
    pub cpu_usage: f32,
    /// The total process-specific memory usage (in bytes) at the time the frame was captured.
    pub memory_usage: u64,
    /// The total process-specific virtual memory usage (in bytes) at the time the frame was captured.
    pub virtual_memory_usage: u64,
    /// The total number of bytes the process has written to disk at the time the frame was captured.
    pub disk_total_written_bytes: u64,
    /// The number of bytes the process has written to disk since the last refresh at the time the frame was captured.
    pub disk_written_bytes: u64,
    /// The total number of bytes the process has read from disk at the time the frame was captured.
    pub disk_total_read_bytes: u64,
    /// The number of bytes the process has read from disk since the last refresh at the time the frame was captured.
    pub disk_read_bytes: u64,
}
```
