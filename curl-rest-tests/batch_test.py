import time
import requests
import statistics
import os

ICE_BASE_URI = os.environ.get("ICE_BASE_URI", "http://localhost:8080")
ENDPOINT = f"{ICE_BASE_URI}/opencds-decision-support-service/api/resources/evaluate"
DATA_FILE = "rest-test-json-evalue.dat"

# Load payload
with open(DATA_FILE, "rb") as f:
    payload = f.read()

headers = {
    "Content-Type": "application/json",
    "Accept": "application/json"
}

times = []
NUM_REQUESTS = 100

print(f"Starting {NUM_REQUESTS} back-to-back evaluate requests to {ENDPOINT}...")

for i in range(NUM_REQUESTS):
    start_time = time.perf_counter()
    response = requests.post(ENDPOINT, headers=headers, data=payload)
    end_time = time.perf_counter()
    
    response.raise_for_status() # Ensure request was successful
    
    elapsed_ms = (end_time - start_time) * 1000
    times.append(elapsed_ms)
    
    if (i + 1) % 10 == 0:
        print(f"Completed {i + 1}/{NUM_REQUESTS} requests...")

total_time_ms = sum(times)
average_time_ms = statistics.mean(times)
min_time_ms = min(times)
max_time_ms = max(times)
median_time_ms = statistics.median(times)

print("\n--- Performance Results ---")
print(f"Total Requests: {NUM_REQUESTS}")
print(f"Total Time:   {total_time_ms:.2f} ms")
print(f"Average Time: {average_time_ms:.2f} ms / request")
print(f"Median Time:  {median_time_ms:.2f} ms")
print(f"Min Time:     {min_time_ms:.2f} ms")
print(f"Max Time:     {max_time_ms:.2f} ms")
