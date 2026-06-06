# LAVA Forecaster REST API Benchmarks

This document records the performance profiles, throughput figures, and latency metrics for the Rust-based LAVA Forecaster HTTP endpoints.

## Benchmark Configuration & Environment

- **CPU Cores**: 16 logical cores (hyperthreaded)
- **Rust Toolchain**: `release` profile, optimized with `jemalloc` allocator enabled
- **Benchmark Type**: Simulated concurrent client load over loopback (`127.0.0.1:8081`)
- **Payload Size**: Single JSON request vs. Bulk JSON / FlatBuffers batch size of **128 patients** per request

---

## 1. Single JSON Endpoint (`/evaluate`)
The single JSON endpoint evaluates one patient record per request.

| Concurrency | Throughput (Reqs/Sec) | Avg Latency | p50 Latency | p90 Latency | p99 Latency |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | 12,140.16 | 0.08 ms | 0.08 ms | 0.09 ms | 0.12 ms |
| **10** | 5,272.42 | 1.88 ms | 1.76 ms | 2.50 ms | 4.88 ms |
| **50** | 4,141.12 | 11.95 ms | 11.20 ms | 16.48 ms | 24.12 ms |

> [!WARNING]
> At higher concurrency (10+ connections) for single-patient requests, network context-switching overhead and locking bottlenecks on the HTTP framework (Axum/Tokio) degrade raw throughput compared to single-threaded performance. For massive evaluation loads, client batching (the Bulk endpoints) should be preferred.

---

## 2. Bulk Endpoint Metrics
Bulk endpoints process a batch of 128 patient evaluations inside a single HTTP request, utilizing CPU-bound parallel processing via Rayon.

### Bulk JSON (`/evaluate_bulk`)
- **Input**: Array of JSON patient objects
- **Output**: Array of JSON vaccine group results

| Concurrency (Simultaneous Requests) | Throughput (Reqs/Sec) | Throughput (Patients/Sec) | Avg Latency | p50 Latency | p90 Latency | p99 Latency |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | 472.14 | 60,433.58 | 2.11 ms | 2.08 ms | 2.46 ms | 3.45 ms |
| **2** | 789.88 | 101,104.97 | 2.52 ms | 2.30 ms | 3.58 ms | 5.01 ms |
| **3** | 1,028.05 | 131,590.98 | 2.91 ms | 2.67 ms | 4.15 ms | 5.91 ms |
| **4** | **1,224.58** | **156,746.09** | 3.25 ms | 3.01 ms | 4.58 ms | 6.49 ms |

---

### Bulk FlatBuffers (`/evaluate_bulk_flatbuffers`)
- **Input**: Octet-stream containing serialized FlatBuffers requests
- **Output**: Octet-stream containing serialized FlatBuffers results
- **Optimizations**: Inline string offset caches, allocation-free CVX/EvaluationReason mappings

| Concurrency (Simultaneous Requests) | Throughput (Reqs/Sec) | Throughput (Patients/Sec) | Avg Latency | p50 Latency | p90 Latency | p99 Latency |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | 806.43 | 103,223.53 | 1.24 ms | 1.14 ms | 1.51 ms | 2.25 ms |
| **2** | 1,153.94 | 147,704.28 | 1.73 ms | 1.47 ms | 2.64 ms | 4.25 ms |
| **3** | **1,507.26** | **192,928.79** | 1.99 ms | 1.76 ms | 2.98 ms | 4.16 ms |
| **4** | 1,506.76 | 192,865.05 | 2.65 ms | 2.10 ms | 3.56 ms | 10.18 ms |

---

## Architectural & Scaling Insights

### Rayon Co-scheduling with Axum
Both JSON and FlatBuffers bulk endpoints use Rayon's `into_par_iter()` internally to parallelize evaluations across the system's 16 cores. 

Because Rayon manages execution using a work-stealing global thread pool:
- **Core Saturation**: Peak evaluation throughput for FlatBuffers is achieved at **Concurrency 3** (~193k patients/sec), completely saturating all 16 execution cores. 
- **Queueing Overhead**: Scaling FlatBuffers to Concurrency 4 does not increase throughput; instead, it introduces queueing delay (p99 latency jumps from **4.16 ms** to **10.18 ms**).
- **Serialization Overhead**: Bulk JSON scales up to Concurrency 4 because a significant portion of its time is spent on synchronous JSON serialization/deserialization on the Tokio workers, leaving execution cores partially idle under lower concurrency levels. FlatBuffers bypasses this, yielding a **71% performance gain** at Concurrency 1 and a **23% performance gain** at peak saturation.
