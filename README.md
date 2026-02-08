# Altaïr Starpaths Microservice

> **Learning path catalog and progression tracking for curated lab sequences**
> 

[![Cloud Run](https://img.shields.io/badge/deploy-Cloud%20Run-blue)](https://cloud.google.com/run)

[![Rust](https://img.shields.io/badge/rust-nightly-orange)](https://www.rust-lang.org)

[![PostgreSQL](336791)](https://www.postgresql.org)

---

## Description

The **Altaïr Starpaths Microservice** manages the catalog of Starpaths—curated learning paths that guide students through sequences of labs. It handles creation, ordering, and tracking user progression through these educational journeys.

This service provides CRUD operations for Starpaths, manages the ordered list of labs within each path, and tracks individual user progress.

**Key capabilities:**

- Create and manage Starpath catalog (name, description, difficulty)
- Order labs within a Starpath with explicit positions
- Track user progression through Starpaths
- Start and monitor learning path completion
- Provide listings and detail views

---

## ⚠️ Security Notice

**This service is currently in MVP stage with NO AUTHENTICATION.**

- ❌ **No auth validation** – All endpoints are publicly accessible
- ❌ **User/creator IDs in request body** – Spoofable without Gateway headers
- ❌ **No ownership checks** – Anyone can modify any Starpath
- ⚠️ **Non-standard GET with body** – Progress endpoint uses JSON body on GET

**Deployment requirement:** Must be migrated to Gateway-based authentication before production.

---

## Architecture

```
┌─────────────┐       ┌──────────────┐       ┌────────────────┐
│  Frontend   │──────▶│   Gateway    │──────▶│  Starpaths MS  │
│             │       │   (TODO)     │       │    (:3005)     │
└─────────────┘       └──────────────┘       └────────┬───────┘
                                                       │
                                                       ▼
                                               ┌───────────────┐
                                               │  PostgreSQL   │
                                               │  (Starpaths)  │
                                               └───────────────┘
                                                 starpaths
                                                 starpath_labs
                                                 user_starpath_progress
```

### Service Flow

1. **Creator creates Starpath** → Defines name, description, difficulty
2. **Creator adds labs** → Assigns labs with explicit positions
3. **Creator reorders labs** → Updates positions as needed
4. **Learner starts Starpath** → Creates progress record
5. **Learner progresses** → Updates current position (TODO: integration with Sessions MS)
6. **Learner completes** → Marks Starpath as completed

---

## Tech Stack

| Component | Technology | Purpose |
| --- | --- | --- |
| **Language** | Rust (nightly) | High-performance async runtime |
| **HTTP Framework** | Axum | HTTP routing and middleware |
| **Async Runtime** | Tokio | Async I/O and concurrency |
| **Database** | PostgreSQL | Starpath and progress persistence |
| **DB Client** | SQLx | Compile-time checked queries |
| **Logging** | tracing + EnvFilter | Structured logging |
| **CI/CD** | GitHub Actions | fmt, clippy, tests |
| **Deployment** | Google Cloud Run | Serverless auto-scaling |

---

## Requirements

### Development

- **Rust** nightly toolchain
- **Docker** & Docker Compose
- **PostgreSQL** 14+ (via `docker compose up postgres`)

### Production (Cloud Run)

- **DATABASE_URL** environment variable (PostgreSQL connection string)
- **PORT** environment variable (default: `3005`)

### Environment Variables

```bash
# Database (required)
DATABASE_URL=postgresql://altair:altair@localhost:5435/altair_starpaths_db

# Server configuration
PORT=3005                                       # Server port (default: 3005)
RUST_LOG=info                                   # Log level filter
```

**⚠️ Database Port Note:** If using `altair-infra` Docker Compose, the Starpaths database is on port `5435`, not the default `5432`.

---

## Installation

### 0. Start infrastructure (database required)

```bash
cd ../altair-infra
docker compose up postgres
```

### 1. Build the Docker image

```bash
cd altair-starpaths-ms
docker build -t altair-starpaths-ms .
```

### 2. Run the service

```bash
docker run --rm -it \
  --network altair-infra_default \
  -p 3005:3005 \
  --env-file .env \
  --name altair-starpaths-ms \
  altair-starpaths-ms
```

**Note:** The service is designed to be destroyed when the terminal closes. Rebuild is necessary for code changes.

---

## Usage

### API Endpoints

#### **GET /health**

Health check for liveness/readiness probes.

**Response:**

```json
{
  "success": true,
  "data": null,
  "meta": {
    "request_id": "...",
    "timestamp": "2026-02-08T17:00:00Z"
  }
}
```

**⚠️ Note:** Unlike other microservices, health endpoint returns `data: null` instead of `{ "status": "ok" }`.

---

#### **GET /starpaths**

List all Starpaths (ordered by creation date, descending).

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "starpath_id": "550e8400-e29b-41d4-a716-446655440000",
      "creator_id": "...",
      "name": "Introduction to Web Security",
      "description": "Learn the basics of web security through hands-on labs",
      "difficulty": "beginner",
      "created_at": "2026-02-08T17:00:00Z"
    }
  ]
}
```

---

#### **POST /starpaths**

Create a new Starpath.

**Request:**

```json
{
  "creator_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Introduction to Web Security",
  "description": "Learn the basics of web security",
  "difficulty": "beginner"
}
```

**⚠️ Security Issue:** `creator_id` is accepted from request body (spoofable). Should be extracted from Gateway headers.

**Response:**

```json
{
  "success": true,
  "data": {
    "starpath_id": "...",
    "creator_id": "...",
    "name": "Introduction to Web Security",
    "description": "...",
    "difficulty": "beginner",
    "created_at": "2026-02-08T17:00:00Z"
  }
}
```

---

#### **GET /starpaths/:id**

Get Starpath details by ID.

**Response:**

```json
{
  "success": true,
  "data": {
    "starpath_id": "...",
    "name": "Introduction to Web Security",
    "description": "...",
    "difficulty": "beginner",
    "created_at": "..."
  }
}
```

**Error (404):**

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Starpath not found"
  }
}
```

---

#### **PUT /starpaths/:id**

Update a Starpath (partial update).

**Request:**

```json
{
  "name": "Advanced Web Security",
  "description": "Updated description",
  "difficulty": "intermediate"
}
```

**Behavior:**

- Uses `COALESCE` for partial updates (omitted fields remain unchanged)
- Returns updated Starpath

**Response:**

```json
{
  "success": true,
  "data": {
    "starpath_id": "...",
    "name": "Advanced Web Security",
    "description": "Updated description",
    "difficulty": "intermediate",
    "created_at": "..."
  }
}
```

---

#### **DELETE /starpaths/:id**

Delete a Starpath.

**Response:**

```json
{
  "success": true,
  "data": {
    "deleted": true
  }
}
```

**Error (404):**

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Starpath not found"
  }
}
```

---

#### **GET /starpaths/:id/labs**

List all labs in a Starpath (ordered by position).

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "starpath_id": "...",
      "lab_id": "550e8400-e29b-41d4-a716-446655440000",
      "position": 1
    },
    {
      "starpath_id": "...",
      "lab_id": "...",
      "position": 2
    }
  ]
}
```

---

#### **POST /starpaths/:id/labs**

Add a lab to a Starpath at a specific position.

**Request:**

```json
{
  "lab_id": "550e8400-e29b-41d4-a716-446655440000",
  "position": 1
}
```

**Response:**

```json
{
  "success": true,
  "data": {
    "starpath_id": "...",
    "lab_id": "...",
    "position": 1
  }
}
```

**Error (409 Conflict):**

```json
{
  "success": false,
  "error": {
    "code": "CONFLICT",
    "message": "Lab already exists in this starpath or position is taken"
  }
}
```

**⚠️ Note:** All SQL errors are converted to 409 Conflict, including unique constraint violations.

---

#### **PUT /starpaths/:id/labs/:lab_id**

Update the position of a lab within a Starpath.

**Request:**

```json
{
  "position": 3
}
```

**Response:**

```json
{
  "success": true,
  "data": {
    "starpath_id": "...",
    "lab_id": "...",
    "position": 3
  }
}
```

**Error (404):**

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Lab not found in starpath"
  }
}
```

---

#### **DELETE /starpaths/:id/labs/:lab_id**

Remove a lab from a Starpath.

**Response:**

```json
{
  "success": true,
  "data": {
    "deleted": true
  }
}
```

---

#### **POST /starpaths/:id/start**

Start a Starpath for a user (idempotent).

**Request:**

```json
"550e8400-e29b-41d4-a716-446655440000"
```

**⚠️ Security Issue:** `user_id` is accepted from request body (spoofable). Should be extracted from Gateway headers.

**Behavior:**

- If progress already exists, returns existing record
- If not, creates new progress with:
    - `current_position = 0`
    - `status = 'in_progress'`
    - `started_at = NOW()`

**Response:**

```json
{
  "success": true,
  "data": {
    "user_id": "...",
    "starpath_id": "...",
    "current_position": 0,
    "status": "in_progress",
    "started_at": "2026-02-08T17:00:00Z",
    "completed_at": null
  }
}
```

---

#### **GET /starpaths/:id/progress**

Get user progress for a Starpath.

**Request:**

```json
"550e8400-e29b-41d4-a716-446655440000"
```

**⚠️ Critical Issue:** Uses JSON body on GET request (non-standard, often blocked by proxies/caches).

**Response:**

```json
{
  "success": true,
  "data": {
    "user_id": "...",
    "starpath_id": "...",
    "current_position": 2,
    "status": "in_progress",
    "started_at": "2026-02-08T16:00:00Z",
    "completed_at": null
  }
}
```

**Error (404):**

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Progress not found"
  }
}
```

---

## Database Schema

### `starpaths` Table

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| `starpath_id` | UUID | PRIMARY KEY | Starpath identifier |
| `creator_id` | UUID | NOT NULL | User who created the Starpath |
| `name` | TEXT | NOT NULL | Starpath name |
| `description` | TEXT | NULLABLE | Starpath description |
| `difficulty` | TEXT | NULLABLE | Difficulty level (beginner/intermediate/advanced) |
| `created_at` | TIMESTAMP | NOT NULL | Creation timestamp |

---

### `starpath_labs` Table

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| `starpath_id` | UUID | NOT NULL | Starpath identifier |
| `lab_id` | UUID | NOT NULL | Lab identifier |
| `position` | INT | NOT NULL | Position in sequence (1-indexed) |

**Constraints:**

- `(starpath_id, lab_id)` – UNIQUE (no duplicate labs)
- `(starpath_id, position)` – UNIQUE (no duplicate positions)

---

### `user_starpath_progress` Table

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| `user_id` | UUID | NOT NULL | User identifier |
| `starpath_id` | UUID | NOT NULL | Starpath identifier |
| `current_position` | INT | NOT NULL | Current position in path |
| `status` | TEXT | NOT NULL | Progress status (`in_progress`, `completed`) |
| `started_at` | TIMESTAMP | NOT NULL | Start timestamp |
| `completed_at` | TIMESTAMP | NULLABLE | Completion timestamp |

**Constraints:**

- `(user_id, starpath_id)` – UNIQUE (one progress per user per Starpath)

---

## Project Structure

```
altair-starpaths-ms/
├── Cargo.toml                    # Rust dependencies
├── Dockerfile                    # Multi-stage build
├── .env                          # Environment variables
├── requests.http                 # HTTP test scenarios
└── src/
    ├── main.rs                  # Server bootstrap, CORS, routes
    ├── state.rs                 # AppState (DB pool + service)
    ├── error.rs                 # AppError type
    ├── routes/
    │   ├── mod.rs              # Route declarations
    │   ├── health.rs           # Health check endpoint
    │   └── starpaths.rs        # All Starpath endpoints
    ├── services/
    │   └── starpaths_service.rs # Core Starpath logic
    └── models/
        ├── starpath.rs         # Starpath data models
        ├── starpath_lab.rs     # Lab positioning models
        └── progress.rs         # Progress tracking models
```

---

## Deployment (Google Cloud Run)

The service is containerized and deployed to **Google Cloud Run** as an internal service.

### Container Configuration

- Listens on port `3005` (configurable via `PORT` env variable)
- Multi-stage Docker build optimizes image size
- Rust nightly toolchain for compilation

### Runtime Requirements

- `DATABASE_URL` environment variable (Cloud SQL or external PostgreSQL)
- Must be deployed in **private network** (no public access in MVP)
- Should be behind authenticated API Gateway (not yet implemented)

### Service Account Permissions

The Cloud Run service account requires:

- Network access to Cloud SQL (or external PostgreSQL)
- No special GCP API permissions required

### Scaling

- Auto-scales based on request load
- Cold start optimized with Rust's fast startup time
- Stateless design enables horizontal scaling

---

## Known Issues & Limitations

### 🔴 Critical Issues

- **No authentication** – All endpoints publicly accessible
- **Spoofable IDs** – `creator_id` and `user_id` accepted from request body
- **GET with body** – Progress endpoint violates HTTP standards

### 🟡 Operational Gaps

- **No ownership validation** – Anyone can modify any Starpath
- **No progression logic** – No integration with Sessions MS to advance position
- **No completion logic** – Status never changes from `in_progress` to `completed`
- **Error handling too broad** – All SQL errors converted to 409 Conflict

### 🟡 Business Logic Limitations

- **No cascade deletes** – Deleting Starpath doesn't clean up labs or progress
- **No ordering helpers** – No "insert at position" that shifts other labs
- **No validation** – Labs can be added that don't exist
- **No difficulty validation** – Accepts any string for difficulty

---

## TODO / Roadmap

### High Priority (MVP → Production)

- [ ]  **Add Gateway authentication** (extract `user_id`, `creator_id` from headers)
- [ ]  **Fix progress endpoint** (remove body from GET, use headers or query params)
- [ ]  **Add ownership checks** (only creator can modify Starpath)
- [ ]  **Add admin role checks** (admins can modify any Starpath)

### Medium Priority (Production Hardening)

- [ ]  **Integrate with Sessions MS** (auto-advance position on lab completion)
- [ ]  **Add completion logic** (mark as completed when all labs done)
- [ ]  **Add ordering helpers** (insert/reorder with automatic position adjustment)
- [ ]  **Add validation** (check labs exist via Labs MS)

### Low Priority (Future Enhancements)

- [ ]  **Add cascade deletes** (clean up labs and progress on Starpath delete)
- [ ]  **Add difficulty validation** (enum or constrained values)
- [ ]  **Add tags/categories** (organize Starpaths by topic)
- [ ]  **Add prerequisites** (require completing Starpath A before B)

---

## Project Status

**⚠️ Current Status: MVP (No Authentication)**

This microservice is **functional for MVP deployment** with core Starpath management operational. Critical authentication gaps must be addressed before production.

**Known limitations to address for production:**

1. Add Gateway-based authentication
2. Fix GET endpoint with body issue
3. Add ownership and role-based access control
4. Integrate progression with Sessions MS
5. Add completion logic
6. Improve error handling specificity

**Maintainers:** Altaïr Platform Team

---

## Notes

- **Port 3005** – Default port, configurable via `PORT` env variable
- **No auth** – MVP accepts IDs from request body (security risk)
- **GET with body** – Non-standard, should be fixed
- **Idempotent start** – Starting multiple times returns same progress record
- **No progression** – Position doesn't auto-advance (requires integration)

---

## License

Internal Altaïr Platform Service – Not licensed for external use.
