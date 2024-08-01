use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::{
    io::BufRead,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// A collection of benchmarks and system specifications.
#[derive(Debug, Serialize)]
pub struct Benchmarks {
    pub system_specs: SystemSpecs,
    pub benchmarks: Vec<Benchmark>,
}

/// Benchmark metadata and phase-specific performance data.
#[derive(Clone, Debug, Serialize)]
pub struct Benchmark {
    /// The name of the benchmark.
    pub name: String,
    /// The path to the benchmark's project folder.
    pub path: PathBuf,
    /// The start time of the benchmark.
    pub start_time: Option<Duration>,
    /// The end time of the benchmark.
    pub end_time: Option<Duration>,
    /// The size of the bytecode of the compiled benchmark.
    pub bytecode_size: Option<usize>,
    /// The phases of the benchmark.
    pub phases: Vec<BenchmarkPhase>,
    /// The performance frames collected from the benchmark.
    pub frames: Arc<Mutex<Vec<BenchmarkFrame>>>,
}

impl Benchmark {
    /// Creates a new benchmark using the supplied `name` and `path`.
    #[inline]
    pub fn new<S: ToString, P: Into<PathBuf>>(name: &S, path: P) -> Self {
        Self {
            name: name.to_string(),
            path: path.into(),
            start_time: None,
            end_time: None,
            bytecode_size: None,
            phases: vec![],
            frames: Arc::new(Mutex::new(Vec::new())).into(),
        }
    }

    /// Runs the benchmark.
    pub fn run(&mut self, epoch: &Instant) {
        // Ensure the benchmark's path is a directory we can run `forc build` in
        assert!(
            self.verify_path(),
            "Project directory \"{}\" does not contain a Toml file.",
            self.path.display()
        );

        // Set the start time of the benchmark
        self.start_time = Some(epoch.elapsed());

        // Spawn the `forc build` child command in the benchmark's directory
        // NOTE: stdin and stdout are piped so that we can use them to signal individual phases
        let mut command = Command::new(
            "/Users/georgiosdelkos/Documents/GitHub/Fuel/forked/sway/target/release/forc",
        )
        .arg("build")
        .arg("--profile-phases")
        .arg("--time-phases")
        .arg("--log-level")
        .arg("5")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .current_dir(self.path.clone())
        .spawn()
        .unwrap();

        // Create an unbounded channel to send/receive line strings between the readline thread and the main thread
        let (readline_tx, readline_rx) = unbounded();

        // Create an unbounded channel to send/receive STOP signals between the readline thread and the main thread
        let (stop_readline_tx, stop_readline_rx) = unbounded();

        // Get the pid of the spawned child command's process
        let pid = sysinfo::Pid::from_u32(command.id());

        // Create a channel to send/receive STOP signals between the perf thread and the main thread
        let (stop_perf_tx, stop_perf_rx) = unbounded();

        Self::spawn_perf_thread(
            epoch,
            pid,
            stop_perf_rx,
            stop_readline_rx.clone(),
            self.frames.clone(),
        );

        // Spawn a thread to read lines from the command's stdout without blocking the main thread
        Self::spawn_readline_thread(&mut command, stop_readline_rx, readline_tx);

        // Collect frames for each phase of the command
        self.wait(
            epoch,
            &mut command,
            &stop_readline_tx,
            &stop_perf_tx,
            &readline_rx,
        );

        // Set the end time of the benchmark
        self.end_time = Some(epoch.elapsed());
    }

    pub fn verify_path(&self) -> bool {
        // Ensure the benchmark's path exists
        if !self.path.exists() {
            return false;
        }

        // Ensure the benchmark's path is a directory
        if !self.path.is_dir() {
            return false;
        }

        // Ensure the benchmark's directory contains a `Forc.toml` file
        let mut toml_path = self.path.clone();
        toml_path.push("Forc.toml");

        if !toml_path.is_file() {
            return false;
        }
        true
    }

    fn spawn_readline_thread(
        command: &mut Child,
        stop_readline_rx: Receiver<()>,
        readline_tx: Sender<String>,
    ) {
        let command_stdout = command.stdout.take().unwrap();

        std::thread::spawn(move || {
            // Wrap the stdout of the child command in a BufReader and move it into the readline thread
            let command_stdout = std::io::BufReader::new(command_stdout);

            for line in command_stdout.lines() {
                let line = line.unwrap().trim_end().to_string();

                // Attempt to send the line to the main thread, or stop looping and allow
                // the readline thread to exit if it fails
                if readline_tx.send(line).is_err() {
                    break;
                }

                // If we receive a STOP signal, stop looping and allow the readline thread to exit
                if stop_readline_rx.try_recv().is_ok() {
                    break;
                }
            }
        });
    }

    fn wait(
        &mut self,
        epoch: &Instant,
        command: &mut Child,
        stop_readline_tx: &Sender<()>,
        stop_perf_tx: &Sender<()>,
        readline_rx: &Receiver<String>,
    ) {
        // Loop until the command has exited
        loop {
            // If the command has exited, tell the readline thread to stop and stop looping
            if command.try_wait().unwrap().is_some() {
                if stop_readline_tx.send(()).is_err() {
                    break;
                }

                if stop_perf_tx.send(()).is_err() {
                    break;
                }

                break;
            }

            // Attempt to receive a line from the readline thread
            let Ok(line) = readline_rx.try_recv() else {
                continue;
            };

            let line = line.trim_start();

            if line.starts_with("/forc-perf start ") {
                // Get the name of the phase from the end of the line
                let name = line.trim_start_matches("/forc-perf start ").trim_end();

                // Add the phase to the current benchmark
                self.phases.push(BenchmarkPhase {
                    name: name.into(),
                    start_time: Some(epoch.elapsed()),
                    end_time: None,
                });
            } else if line.starts_with("/forc-perf stop ") {
                // Get the name of the phase from the end of the line
                let name = line.trim_start_matches("/forc-perf stop ").trim_end();

                // Get the current benchmark phase
                let phase = self
                    .phases
                    .iter_mut()
                    .rev()
                    .find(|phase| name == phase.name)
                    .unwrap();

                // Ensure the received name matches the name of the current phase
                assert!(
                    name == phase.name,
                    "Received phase name \"{}\" does not match current phase name \"{}\"",
                    name,
                    phase.name,
                );

                // Set the end time of the benchmark
                phase.end_time = Some(epoch.elapsed());
            } else if line.starts_with("/forc-perf size ") {
                // Parse the size of the bytecode compiled for the benchmark code from the end of the line
                self.bytecode_size = Some(
                    line.trim_start_matches("/forc-perf size ")
                        .trim_end()
                        .parse()
                        .unwrap()
                );
            }
        }
    }

    fn spawn_perf_thread(
        epoch: &Instant,
        pid: sysinfo::Pid,
        stop_perf_rx: Receiver<()>,
        stop_readline_rx: Receiver<()>,
        frames: Arc<Mutex<Vec<BenchmarkFrame>>>,
    ) {
        let epoch = epoch.clone();

        let mut system = sysinfo::System::new();

        let num_cpus = {
            system.refresh_cpu_all();
            system.cpus().len()
        };

        let refresh_kind = sysinfo::ProcessRefreshKind::new()
            .with_cpu()
            .with_memory()
            .with_disk_usage();

        std::thread::spawn(move || loop {
            let frame_start = std::time::Instant::now();

            // If we receive a STOP signal, stop looping and allow the perf thread to exit
            if stop_perf_rx.try_recv().is_ok() {
                break;
            }

            if stop_readline_rx.try_recv().is_ok() {
                break;
            }

            if system
                .refresh_processes_specifics(sysinfo::ProcessesToUpdate::Some(&[pid]), refresh_kind)
                != 1
            {
                break;
            }

            let Some(process) = system.process(pid) else {
                panic!("Failed to find process with pid {pid}");
            };

            let cpu_usage = process.cpu_usage() / num_cpus as f32;
            let memory_usage = process.memory();
            let virtual_memory_usage = process.virtual_memory();
            let disk_usage = process.disk_usage();

            frames.lock().unwrap().push(BenchmarkFrame {
                timestamp: frame_start.duration_since(epoch),
                cpu_usage,
                memory_usage,
                virtual_memory_usage,
                disk_total_written_bytes: disk_usage.total_written_bytes,
                disk_written_bytes: disk_usage.written_bytes,
                disk_total_read_bytes: disk_usage.total_read_bytes,
                disk_read_bytes: disk_usage.read_bytes,
            });

            let frame_elapsed = frame_start.elapsed();

            // Ensure that we don't loop any faster than the minimum frame duration
            if frame_elapsed < BenchmarkFrame::MINIMUM_DURATION {
                std::thread::sleep(BenchmarkFrame::MINIMUM_DURATION - frame_elapsed);
            }
        });
    }
}

/// A named collection of performance frames representing a single phase of a benchmark.
#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkPhase {
    /// The name of the benchmark phase.
    pub name: String,
    /// The start time of the benchmark phase.
    pub start_time: Option<Duration>,
    /// The end time of the benchmark phase.
    pub end_time: Option<Duration>,
}

impl BenchmarkFrame {
    /// The minimum duration of a performance frame.
    pub const MINIMUM_DURATION: Duration = Duration::from_millis(100);
}

/// A single frame of performance information for a benchmark phase.
#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkFrame {
    /// The time that the frame was captured.
    pub timestamp: Duration,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemSpecs {
    #[serde(skip_serializing)]
    pub global_cpu_usage: f64,
    pub cpus: Vec<Cpu>,
    pub physical_core_count: i64,
    pub total_memory: i64,
    pub free_memory: i64,
    pub available_memory: i64,
    pub used_memory: i64,
    pub total_swap: i64,
    pub free_swap: i64,
    pub used_swap: i64,
    pub uptime: i64,
    pub boot_time: i64,
    pub load_average: LoadAverage,
    pub name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub long_os_version: String,
    pub distribution_id: String,
    pub host_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cpu {
    #[serde(skip_serializing)]
    pub cpu_usage: f64,
    pub name: String,
    pub vendor_id: String,
    pub brand: String,
    pub frequency: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}
