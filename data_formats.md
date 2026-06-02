# LAVA Forecaster: API Data Formats Reference

This document describes the JSON data formats used by the REST API and command-line execution interfaces in the Lightspeed Antigen & Vaccine Assessment (LAVA) Forecaster.

---

## 1. Single Patient Request Format

Endpoints:
* `POST /evaluate` (Standard endpoint, supports this simplified JSON schema)
* CLI: `cargo run -- <path_to_request.json>`

### Schema Definition
```json
{
  "patient": {
    "birth_date": "YYYY-MM-DD",
    "gender": "female" | "male" | "unknown",
    "immunities": [
      {
        "disease": "HepB" | "Varicella" | "Measles" | "Mumps" | "Rubella" | "HepA" | "...",
        "date": "YYYY-MM-DD",
        "reason": "Titer positive" | "..."
      }
    ],
    "contraindications": [
      {
        "date": "YYYY-MM-DD",
        "target": "DTP" | "DTaP" | "..." | "cvx <number>",
        "reason": "Contraindication reason",
        "valid_until": "YYYY-MM-DD" | null,
        "cvx": "CVX code string" | null
      }
    ]
  },
  "history": [
    {
      "date": "YYYY-MM-DD",
      "cvx": "CVX code string" (e.g., "03", "110"),
      "is_valid": true | false | null
    }
  ],
  "execution_date": "YYYY-MM-DD"
}
```

### Example Request (`request.json`)
```json
{
  "patient": {
    "birth_date": "2024-01-01",
    "gender": "female",
    "immunities": [
      {
        "disease": "HepB",
        "date": "2024-06-01",
        "reason": "Documented disease"
      }
    ],
    "contraindications": [
      {
        "date": "2024-03-01",
        "target": "DTP",
        "reason": "Severe allergic reaction"
      }
    ]
  },
  "history": [
    {
      "date": "2024-03-01",
      "cvx": "08",
      "is_valid": null
    }
  ],
  "execution_date": "2026-06-02"
}
```

---

## 2. Bulk Request Format

Endpoint: `POST /evaluate_bulk`

The bulk request simply wraps multiple single request objects inside a `requests` array wrapper.

```json
{
  "requests": [
    {
      "patient": { "birth_date": "2024-01-01", "gender": "female" },
      "history": [],
      "execution_date": "2026-06-02"
    }
  ]
}
```

---

## 3. Forecast Response Format

This structure is returned in the HTTP response body for a single evaluation.

### Schema Definition
```json
{
  "vaccine_groups": [
    {
      "vaccine_group": "string" (e.g. "POLIO", "HEP_B", "MMR"),
      "selected_series": "string" | null,
      "evaluations": [
        {
          "dose_date": "YYYY-MM-DD",
          "cvx": "string" (e.g. "10", "110"),
          "status": "Valid" | "Invalid" | "Accepted" | "Ignored",
          "reasons": [
            "BelowMinimumAge" | "BelowMinimumInterval" | "DuplicateShotSameDay" | "..."
          ],
          "dose_number": 1 | 2 | 3 | 4 | null
        }
      ],
      "forecasts": [
        {
          "series_name": "string",
          "status": "NotComplete" | "Complete" | "NotRecommended" | "ConditionallyRecommended",
          "earliest_date": "YYYY-MM-DD" | null,
          "recommended_date": "YYYY-MM-DD" | null,
          "overdue_date": "YYYY-MM-DD" | null,
          "latest_date": "YYYY-MM-DD" | null,
          "reasons": ["string"]
        }
      ]
    }
  ]
}
```

### Example Response
```json
{
  "vaccine_groups": [
    {
      "vaccine_group": "VARICELLA",
      "selected_series": "VARICELLA_2_DOSE_SERIES",
      "evaluations": [
        {
          "dose_date": "2025-01-01",
          "cvx": "21",
          "status": "Valid",
          "reasons": [],
          "dose_number": 1
        }
      ],
      "forecasts": [
        {
          "series_name": "VARICELLA_2_DOSE_SERIES",
          "status": "NotComplete",
          "earliest_date": "2025-03-26",
          "recommended_date": "2028-01-01",
          "overdue_date": "2031-01-29",
          "latest_date": null,
          "reasons": []
        }
      ]
    }
  ]
}
```

---

## 4. Bulk Response Format

Endpoint: `POST /evaluate_bulk` output.

Wraps multiple single `ForecastResponse` items:
```json
{
  "responses": [
    {
      "vaccine_groups": [...]
    }
  ]
}
```
