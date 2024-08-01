# Forc-Perf

## Description

This project is a profiler for the Sway compiler. It is designed to be lightweight and collect poison free data from the compiler.
It collects frames of data and timestamps for all the different compiler passes and the time it takes to run them. It also collects the time it takes to run the entire compilation.

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
