import time
import requests
import statistics
import os

JAVA_ENDPOINT = "http://localhost:8080/opencds-decision-support-service/api/resources/evaluate"
RUST_ENDPOINT = "http://localhost:8081/evaluate"
DATA_FILE = "rest-test-json-evalue.dat"

# Load payload
with open(DATA_FILE, "rb") as f:
    payload = f.read()

headers = {
    "Content-Type": "application/json",
    "Accept": "application/json"
}

NUM_REQUESTS = 100

def run_benchmark(name, endpoint):
    print(f"\nStarting benchmark for {name} ({endpoint})...")
    times = []
    for i in range(NUM_REQUESTS):
        start_time = time.perf_counter()
        response = requests.post(endpoint, headers=headers, data=payload)
        end_time = time.perf_counter()
        
        response.raise_for_status()
        
        elapsed_ms = (end_time - start_time) * 1000
        times.append(elapsed_ms)
        
        if (i + 1) % 25 == 0:
            print(f"  Completed {i + 1}/{NUM_REQUESTS} requests...")
            
    return {
        "total": sum(times),
        "avg": statistics.mean(times),
        "median": statistics.median(times),
        "min": min(times),
        "max": max(times)
    }

# 1. Warm-up both services with a few requests
print("Warming up endpoints...")
for _ in range(5):
    try:
        requests.post(JAVA_ENDPOINT, headers=headers, data=payload).raise_for_status()
    except Exception as e:
        print(f"Warning: Java service warmup failed: {e}")
    try:
        requests.post(RUST_ENDPOINT, headers=headers, data=payload).raise_for_status()
    except Exception as e:
        print(f"Warning: Rust service warmup failed: {e}")

# 2. Run benchmarks
java_results = run_benchmark("Java ICE service", JAVA_ENDPOINT)
rust_results = run_benchmark("Rust PoC service", RUST_ENDPOINT)

# 3. Print Results
print("\n" + "=" * 60)
print(f"{'Metric':<15} | {'Java ICE Service':<18} | {'Rust PoC Service':<18}")
print("-" * 60)
print(f"{'Total Time':<15} | {java_results['total']:>14.2f} ms | {rust_results['total']:>14.2f} ms")
print(f"{'Average Time':<15} | {java_results['avg']:>14.2f} ms | {rust_results['avg']:>14.2f} ms")
print(f"{'Median Time':<15} | {java_results['median']:>14.2f} ms | {rust_results['median']:>14.2f} ms")
print(f"{'Min Time':<15} | {java_results['min']:>14.2f} ms | {rust_results['min']:>14.2f} ms")
print(f"{'Max Time':<15} | {java_results['max']:>14.2f} ms | {rust_results['max']:>14.2f} ms")
print("=" * 60)
print(f"Speedup: {java_results['avg'] / rust_results['avg']:.2f}x faster on average.")
