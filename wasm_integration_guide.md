# WebAssembly (WASM) Integration Guide

The LAVA Forecaster engine can compile to WebAssembly, enabling high-performance immunization forecasting and CDSi evaluation directly within web browsers, web workers, and Node.js environments without needing a backend server.

---

## 1. Prerequisites

Make sure you have:
- **Rust** toolchain installed (edition 2024 or latest).
- **Node.js** and **npm** installed.
- The `wasm32-unknown-unknown` target added to Rust:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

---

## 2. Building the WASM Package

We use `wasm-pack` to build the compiled WASM binary and generate its corresponding JavaScript/TypeScript bindings. Choose the build command based on your target environment:

### A. Targeting Node.js (CommonJS)
Useful for server-side JS environments, microservices, or local scripting:
```bash
npx wasm-pack build --target nodejs
```

### B. Targeting Web Browsers (Native ES Modules)
Useful when loading WASM dynamically using native `<script type="module">`:
```bash
npx wasm-pack build --target web
```

### C. Targeting Bundlers (Vite, Webpack, Rollup)
Useful for frameworks like React, Next.js, Vue, or Svelte:
```bash
npx wasm-pack build --target bundler
```

The output will be placed in the `pkg/` directory at the project root, containing:
- `lava_forecaster_bg.wasm`: The compiled WebAssembly binary.
- `lava_forecaster.js`: The JavaScript wrapper bindings.
- `lava_forecaster.d.ts`: TypeScript definition files.
- `package.json`: A ready-to-publish npm package configuration.

---

## 3. Exposed JavaScript API

The WASM build exposes two primary functions from `wasm_bindgen`:

### `evaluate_patient(request_json: string): string`
Accepts a JSON string containing the patient record and history, runs the forecast engine, and returns a JSON string containing the forecast results.

### `evaluate_patient_js(request: ForecastRequest): ForecastResponse`
Accepts a JavaScript object and returns a JavaScript object. This handles serialization/deserialization seamlessly using `serde-wasm-bindgen`.

### Type Definitions

```typescript
export interface Patient {
  birth_date: string; // YYYY-MM-DD
  gender: "Female" | "Male" | "Unknown";
}

export interface Dose {
  date: string; // YYYY-MM-DD
  cvx: string | number; // CDC CVX code
}

export interface ForecastRequest {
  patient: Patient;
  history: Dose[];
  execution_date: string; // YYYY-MM-DD
}

// Response structure containing vaccine group evaluations and forecasts
export interface ForecastResponse {
  vaccine_groups: VaccineGroupForecast[];
}
```

---

## 4. Usage Examples

### A. Node.js (CommonJS)
```javascript
const pkg = require('./pkg/lava_forecaster.js');

const request = {
  patient: { birth_date: "2020-01-01", gender: "Female" },
  history: [{ date: "2020-03-01", cvx: "10" }],
  execution_date: "2026-05-20"
};

// Option 1: Object-based API (Recommended)
const response = pkg.evaluate_patient_js(request);
console.log("Groups forecasted:", response.vaccine_groups.length);

// Option 2: JSON string API
const responseJson = pkg.evaluate_patient(JSON.stringify(request));
const responseObj = JSON.parse(responseJson);
```

### B. In the Browser (Native ES Modules)
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>LAVA Forecaster Browser Demo</title>
</head>
<body>
  <script type="module">
    import init, { evaluate_patient_js } from './pkg/lava_forecaster.js';

    async function run() {
      // 1. Initialize the WASM module
      await init();

      const request = {
        patient: { birth_date: "2020-01-01", gender: "Female" },
        history: [
          { date: "2020-03-01", cvx: "10" },
          { date: "2020-05-01", cvx: "10" }
        ],
        execution_date: "2026-05-20"
      };

      // 2. Perform forecasting
      const response = evaluate_patient_js(request);
      console.log("Evaluation Results:", response);
    }

    run();
  </script>
</body>
</html>
```

### C. In Bundler Projects (e.g., Vite + React/Svelte)
For frameworks bundled with Vite, you may need a plugin like `vite-plugin-wasm` or load it dynamically:
```typescript
import init, { evaluate_patient_js } from 'lava-forecaster-wasm';

async function calculateForecast() {
  await init();
  const response = evaluate_patient_js({
    patient: { birth_date: "2020-01-01", gender: "Male" },
    history: [],
    execution_date: "2026-05-20"
  });
  return response;
}
```
