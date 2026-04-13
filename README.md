````md
# 🚀 Backend Wizards — Stage 0 Task  
## API Integration & Data Processing Assessment

---

## 📌 Overview

This project is a backend API built for the **Backend Wizards Stage 0 assessment**.  
It demonstrates external API integration, data transformation, and structured response handling using a single REST endpoint.

The service fetches gender prediction data from the **Genderize API**, processes it, and returns a clean, standardized response.

---

## 🎯 Objective

Build a single GET endpoint that:

- Accepts a `name` query parameter
- Calls the external **Genderize API**
- Processes and transforms the response
- Returns structured JSON output based on defined rules

---

## 🔗 External API Used

- 🌐 Genderize API: https://api.genderize.io

---

## 🛠️ Tech Stack

- Rust 🦀 (Axum Framework)
- Reqwest (HTTP client)
- Tokio (Async runtime)
- Serde (Serialization)
- Tower HTTP (CORS handling)
- Chrono (Date & time)

---

## 📡 API Endpoint

### Classify Name

```http
GET /api/classify?name={name}
````

---

## 📥 Query Parameters

| Parameter | Type   | Required | Description      |
| --------- | ------ | -------- | ---------------- |
| name      | string | Yes      | Name to classify |

---

## 📤 Success Response (200 OK)

```json
{
  "status": "success",
  "data": {
    "name": "john",
    "gender": "male",
    "probability": 0.99,
    "sample_size": 1234,
    "is_confident": true,
    "processed_at": "2026-04-01T12:00:00Z"
  }
}
```

---

## ⚙️ Data Processing Rules

### Field Mapping

* `count` → `sample_size`

---

### Confidence Logic

```
is_confident = true IF:
probability >= 0.7 AND sample_size >= 100
```

Both conditions must be true.

---

### Timestamp Generation

* Field: `processed_at`
* Format: UTC ISO 8601
* Generated dynamically per request

Example:

```
2026-04-01T12:00:00Z
```

---

##  Error Responses

All errors follow this format:

```json
{
  "status": "error",
  "message": "error description"
}
```

---

###  400 Bad Request

Missing or empty `name` parameter:

```json
{
  "status": "error",
  "message": "name parameter is required and cannot be empty"
}
```

---

###  422 Unprocessable Entity

No prediction available:

```json
{
  "status": "error",
  "message": "No prediction available for the provided name"
}
```

---

###  502 Bad Gateway

External API failure:

```json
{
  "status": "error",
  "message": "Failed to call external API"
}
```

---

###  500 Internal Server Error

Unexpected server failure:

```json
{
  "status": "error",
  "message": "Internal server error"
}
```

---

##  Genderize Edge Case Handling

If the API returns:

* `gender: null` OR
* `count: 0`

Response:

```json
{
  "status": "error",
  "message": "No prediction available for the provided name"
}
```

---

##  CORS Policy

This API allows all origins:

```
Access-Control-Allow-Origin: *
```

Required for grading compatibility.

---

##  Performance Requirements

* Response time: < 500ms (excluding external API latency)
* Must handle multiple concurrent requests
* Must remain stable under load

---

##  Example Request

```bash
curl "https://your-api-url.com/api/classify?name=john"
```

---

##  Example Response

```json
{
  "status": "success",
  "data": {
    "name": "john",
    "gender": "male",
    "probability": 0.99,
    "sample_size": 1234,
    "is_confident": true,
    "processed_at": "2026-04-01T12:00:00Z"
  }
}
```

---

##  Evaluation Criteria (100 Points)

| Category                    | Points |
| --------------------------- | ------ |
| Endpoint Availability       | 10     |
| Query Parameter Handling    | 10     |
| External API Integration    | 20     |
| Data Extraction Accuracy    | 15     |
| Confidence Logic            | 15     |
| Error Handling              | 10     |
| Edge Case Handling          | 10     |
| Response Format & Structure | 10     |

---

## 🚀 Deployment

You can deploy using:

* Railway
* Vercel
* AWS
* Heroku
* Any production-ready platform

 Render is not accepted

---

## Fly.io Deployment

This repository is now configured for Fly.io with:

- `fly.toml` (service config and health checks)
- `Dockerfile` (multi-stage Rust build + CA certificates)
- App binding to `PORT` with fallback to `8080`
- Health endpoint at `GET /health`

Deploy steps:

```bash
fly auth login
fly launch --no-deploy
fly deploy
```

Notes:

- If `fly launch` suggests a different app name, update the `app` value in `fly.toml`.
- The app serves HTTP internally on port `8080` as expected by Fly config.

---

##  Repository Requirements

Your GitHub repo must include:

* Source code
* This README.md
* Setup instructions
* Dependencies (Cargo.toml)
* Clean project structure

---

##  Testing Checklist

* [ ] Endpoint is live
* [ ] Works across networks
* [ ] Correct JSON format
* [ ] Handles empty name
* [ ] Handles external API failure
* [ ] CORS enabled
* [ ] Stable under load

---

##  Submission Steps

1. Confirm API is live
2. Test endpoint externally
3. Submit:

   * API base URL
   * GitHub repo link
   * Full name
   * Email
   * Stack used
4. Wait for grading bot response

---

##  Deadline

**Sunday, 13th April 2026 — 11:59pm (WAT)**

---

##  Final Note

This project demonstrates:

* API integration
* Data processing
* Error handling
* Async backend development
* Production-ready API design

```
```
