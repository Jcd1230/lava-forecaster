# LAVA Forecaster vs. Java ICE Benchmarks

This document records the performance profiles, throughput figures, and latency metrics for both the Rust-based LAVA Forecaster and the legacy Drools-based Java ICE engine, highlighting in-process engine speed vs. REST serialization overhead.

## Benchmark Environment
- **CPU Cores**: 16 logical cores (hyperthreaded)
- **Rust Toolchain**: `release` profile, optimized with `jemalloc` allocator enabled
- **Java Toolchain**: Java 25, G1 Garbage Collector enabled, heap pre-allocated (`-Xms2g -Xmx4g`)
- **Payload**: Standard CDSi test suite of **1,155 patient cases** (averaging 2.17 historical doses per patient, ranging from 0 to 7 doses)

---

## 1. Engine Core Performance (In-Process)
This benchmark measures raw CDSi rules evaluation speed, excluding HTTP server context-switching, network latency, and JSON serialization.

### LAVA Forecaster (Rust Crate Core)
*Measured via `cargo run --release --bin benchmark`*

| Workload Scenario | Threading | Throughput (Evals/Sec) | Average Latency |
| :--- | :--- | :--- | :--- |
| **Full Suite (1,155 cases)** | Single-Threaded | 95,628.61 | 10.46 microseconds / patient |
| | Multi-Threaded | **850,800.84** | **1.18 microseconds** (effective) |
| **Minimal Case (0 doses)** | Single-Threaded | 173,739.49 | 5.76 microseconds / patient |
| | Multi-Threaded | 1,591,598.58 | 0.63 microseconds (effective) |
| **Complex Case (7 doses)** | Single-Threaded | 58,479.22 | 17.10 microseconds / patient |
| | Multi-Threaded | 516,705.64 | 1.94 microseconds (effective) |

---

## 2. REST API Endpoint Performance
HTTP request throughput and latencies include transport overhead, network I/O, and payload serialization.

### Java ICE vs. LAVA REST Bulk Endpoints
*Bulk operations process batches of **128 patients** per request. Rust benchmarks run on `127.0.0.1:8081`.*

| Service / API Protocol | Concurrency | Request Throughput | Patient Throughput | Average Latency |
| :--- | :--- | :--- | :--- | :--- |
| **Java ICE Monolith** (REST JSON) | Single-Threaded | 11.58 reqs/sec | 1,482 patients/sec | ~86.2 ms |
| | Multi-Threaded | **138.98 reqs/sec** | **17,789 patients/sec** | ~8.31 ms (effective) |
| **LAVA REST JSON** (`/evaluate_bulk`) | Concurrency 1 | 472.14 reqs/sec | 60,433.58 patients/sec | 2.11 ms |
| | Concurrency 2 | 789.88 reqs/sec | 101,104.97 patients/sec | 2.52 ms |
| | Concurrency 3 | 1,028.05 reqs/sec | 131,590.98 patients/sec | 2.91 ms |
| | Concurrency 4 | **1,224.58 reqs/sec** | **156,746.09 patients/sec** | 3.25 ms |
| **LAVA FlatBuffers** (`/evaluate_bulk_flatbuffers`) | Concurrency 1 | 806.43 reqs/sec | 103,223.53 patients/sec | 1.24 ms |
| | Concurrency 2 | 1,153.94 reqs/sec | 147,704.28 patients/sec | 1.73 ms |
| | Concurrency 3 | **1,507.26 reqs/sec** | **192,928.79 patients/sec** | 1.99 ms |
| | Concurrency 4 | 1,506.76 reqs/sec | 192,865.05 patients/sec | 2.65 ms |

> [!NOTE]
> **Performance Scaling Observations**:
> - **Core Saturation**: The Bulk FlatBuffers endpoint peaks at **Concurrency 3** (~193k patients/sec) on a 16-core system and hits a hard CPU limit. Adding a 4th worker introduces scheduling queue delays (p99 latency spikes from **4.16 ms** to **10.18 ms**).
> - **Serialization Bottlenecks**: JSON continues scaling up to Concurrency 4 because the worker threads spend significant time on CPU-bound string serialization/deserialization, keeping the Rayon evaluation pool only partially utilized at lower connection counts.

---

## Performance Summary & Architectural Insights

1. **Evaluation Engine Core Speed**: The LAVA Rust evaluation engine performs an individual patient forecasting suite in **10.4 microseconds** (single-threaded) on the CPU. At this tier of performance, the engine itself is functionally instantaneous; production service performance is dominated entirely by network I/O and data serialization format.
2. **Bulk Serialization Scenarios**: 
   - **JSON Overhead**: Moving from raw in-process Rust evaluation to HTTP JSON REST wraps the engine in serialization/deserialization logic. In a bulk request, JSON parsing/generation accounts for **~40-50% of total latency**.
   - **FlatBuffers Advantage**: FlatBuffers reduces deserialization overhead to near-zero and provides structured serialization, yielding a **71% throughput increase** at single-concurrency (103k vs 60k patients/sec) and a **23% increase** at core saturation (193k vs 156k patients/sec).
3. **Rust LAVA vs. Optimized Java ICE**: Comparing the optimized Java ICE engine (using supporting-data caching and JVM/pooling tune-ups) to LAVA REST JSON:
   - **Single-Threaded Speedup**: LAVA JSON processes **60,433 patients/sec** vs. Java ICE's **1,482 patients/sec** (**~40x speedup**).
   - **Peak Multi-Threaded Speedup**: LAVA FlatBuffers processes **192,928 patients/sec** vs. Java ICE's **17,789 patients/sec** (**~11x speedup**).
