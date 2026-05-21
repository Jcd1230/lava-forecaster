import time
import requests
import statistics
import socket
import json

RUST_HOST = "localhost"
RUST_PORT = 8081

RUST_EVALUATE_PATH = "/evaluate"
RUST_BULK_PATH = "/evaluate_bulk"

PATIENT_REQUEST = {
    "patient": {
        "birth_date": "1999-06-10",
        "gender": "Female"
    },
    "history": [
        { "date": "1999-10-16", "cvx": "10" },
        { "date": "2006-04-11", "cvx": "110" },
        { "date": "2009-07-21", "cvx": "110" },
        { "date": "2009-07-21", "cvx": "120" },
        { "date": "2016-03-15", "cvx": "10" }
    ],
    "execution_date": "2016-03-15"
}

class RawTCPClient:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.sock = None

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.sock.connect((self.host, self.port))

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except Exception:
                pass
            self.sock = None

    def post(self, path, body):
        if self.sock is None:
            self.connect()
            
        req_lines = [
            f"POST {path} HTTP/1.1",
            f"Host: {self.host}:{self.port}",
            "Content-Type: application/json",
            f"Content-Length: {len(body)}",
            "Connection: keep-alive",
            "",
            body
        ]
        req_bytes = "\r\n".join(req_lines).encode("utf-8")
        
        for attempt in range(2):
            try:
                self.sock.sendall(req_bytes)
                
                # Read headers
                resp_bytes = b""
                while b"\r\n\r\n" not in resp_bytes:
                    chunk = self.sock.recv(8192)
                    if not chunk:
                        raise ConnectionError("Connection closed by server")
                    resp_bytes += chunk
                    
                headers_part, body_part = resp_bytes.split(b"\r\n\r\n", 1)
                
                content_length = 0
                process_time_us = 0
                for line in headers_part.decode("utf-8").split("\r\n")[1:]:
                    if ":" in line:
                        k, v = line.split(":", 1)
                        k = k.strip().lower()
                        v = v.strip()
                        if k == "content-length":
                            content_length = int(v)
                        elif k == "x-process-time-us":
                            process_time_us = int(v)
                            
                while len(body_part) < content_length:
                    chunk = self.sock.recv(8192)
                    if not chunk:
                        raise ConnectionError("Connection closed while reading body")
                    body_part += chunk
                    
                return body_part.decode("utf-8"), process_time_us
            except Exception:
                self.close()
                if attempt == 1:
                    raise
                self.connect()

def run_requests_benchmark(batch_size, total_patients):
    num_requests = total_patients // batch_size
    client_rtts = []
    server_times = []
    
    session = requests.Session()
    
    if batch_size == 1:
        payload = json.dumps(PATIENT_REQUEST)
        url = f"http://{RUST_HOST}:{RUST_PORT}{RUST_EVALUATE_PATH}"
    else:
        payload = json.dumps({
            "requests": [PATIENT_REQUEST] * batch_size
        })
        url = f"http://{RUST_HOST}:{RUST_PORT}{RUST_BULK_PATH}"
        
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    
    # Warmup
    for _ in range(3):
        try:
            session.post(url, data=payload, headers=headers).raise_for_status()
        except Exception:
            pass
            
    for _ in range(num_requests):
        t0 = time.perf_counter()
        resp = session.post(url, data=payload, headers=headers)
        t1 = time.perf_counter()
        
        resp.raise_for_status()
        client_rtts.append((t1 - t0) * 1000.0) # in ms
        
        server_us = int(resp.headers.get("X-Process-Time-Us", 0))
        server_times.append(server_us / 1000.0) # in ms
        
    total_client_ms = sum(client_rtts)
    total_server_ms = sum(server_times)
    
    avg_client_patient = (total_client_ms / total_patients) * 1000.0 # in microseconds
    avg_server_patient = (total_server_ms / total_patients) * 1000.0 # in microseconds
    
    return {
        "total_client_ms": total_client_ms,
        "total_server_ms": total_server_ms,
        "avg_client_patient_us": avg_client_patient,
        "avg_server_patient_us": avg_server_patient,
        "throughput": total_patients / (total_client_ms / 1000.0)
    }

def run_raw_socket_benchmark(batch_size, total_patients):
    num_requests = total_patients // batch_size
    client_rtts = []
    server_times = []
    
    client = RawTCPClient(RUST_HOST, RUST_PORT)
    
    if batch_size == 1:
        payload = json.dumps(PATIENT_REQUEST)
        path = RUST_EVALUATE_PATH
    else:
        payload = json.dumps({
            "requests": [PATIENT_REQUEST] * batch_size
        })
        path = RUST_BULK_PATH
        
    # Warmup
    for _ in range(3):
        try:
            client.post(path, payload)
        except Exception:
            pass
            
    for _ in range(num_requests):
        t0 = time.perf_counter()
        _, server_us = client.post(path, payload)
        t1 = time.perf_counter()
        
        client_rtts.append((t1 - t0) * 1000.0) # in ms
        server_times.append(server_us / 1000.0) # in ms
        
    client.close()
    
    total_client_ms = sum(client_rtts)
    total_server_ms = sum(server_times)
    
    avg_client_patient = (total_client_ms / total_patients) * 1000.0 # in microseconds
    avg_server_patient = (total_server_ms / total_patients) * 1000.0 # in microseconds
    
    return {
        "total_client_ms": total_client_ms,
        "total_server_ms": total_server_ms,
        "avg_client_patient_us": avg_client_patient,
        "avg_server_patient_us": avg_server_patient,
        "throughput": total_patients / (total_client_ms / 1000.0)
    }

def print_results(results):
    print("| Batch Size | Total Patients | Client RTT (Total) | Server Proc (Total) | Avg Client/Pat | Avg Server/Pat | Throughput |")
    print("| ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for batch_size, res in results.items():
        print(f"| {batch_size:4d} | {res['patients']:14d} | {res['total_client_ms']:14.2f} ms | {res['total_server_ms']:15.2f} ms | {res['avg_client_patient_us']:12.2f} μs | {res['avg_server_patient_us']:12.2f} μs | {res['throughput']:10.1f}/s |")

def main():
    print("==========================================================================")
    print("               Rust ICE Forecaster Bulk request Benchmark                 ")
    print("==========================================================================")
    
    TOTAL_PATIENTS = 1000
    
    # 1. Benchmark standard requests library
    print("\n### 1. Standard Python `requests` Library Client")
    print("Note: Splits header and body writes, exposing Nagle's/Delayed ACK delay on Loopback for mid-sized requests.")
    results_requests = {}
    for size in [1, 10, 100, 1000]:
        results_requests[size] = run_requests_benchmark(size, TOTAL_PATIENTS)
        results_requests[size]["patients"] = TOTAL_PATIENTS
    print_results(results_requests)
    
    # 2. Benchmark raw TCP sockets with sendall()
    print("\n### 2. Optimized Raw TCP Socket Client (`sendall`)")
    print("Note: Writes header and body in a single packet, bypassing Nagle's/Delayed ACK delay.")
    results_raw = {}
    for size in [1, 10, 100, 1000]:
        results_raw[size] = run_raw_socket_benchmark(size, TOTAL_PATIENTS)
        results_raw[size]["patients"] = TOTAL_PATIENTS
    print_results(results_raw)

if __name__ == "__main__":
    main()
