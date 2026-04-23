# merk

A blazing-fast, foundational Rust web service built with [Axum](https://github.com/tokio-rs/axum).

This project establishes an enterprise-grade architecture out of the box, equipped with REST protocols, GraphQL interfaces, API documentation generation, metrics extraction, and robust typed error handling.

## Features

- **Robust Framework**: Powered by Tokio and Axum for high-performance asynchronous execution.
- **REST & GraphQL Dual Support**: First-class handling of traditional REST routes alongside `async-graphql` schemas.
- **Auto-Generating OpenAPI Docs**: Routes mapped using `aide` automatically reflect in a beautiful Scalar-driven UI (equipped with pre-configured JWT Bearer authentication).
- **SurrealDB with Auto-Migrations**: Schema-enforced tables with an embedded migration runner that applies `.surql` files at startup — no external migration tool needed.
- **RBAC via Graph Traversal**: Role and permission assignment stored as native SurrealDB graph edges, queryable via graph traversal syntax.
- **Infrastructure Monitoring**: Tightly integrated with Prometheus, scraping application telemetry at `/metrics`.
- **Intelligent Configuration**: Uses `envy` to map environment variables directly into a strongly typed `AppConfig` state securely backing dependency injection architectures.
- **Dynamic TLS Generation**: A toggleable HTTPS binding that spins up a virtual `rcgen` self-signed certificate natively on boot.
- **Granular Error Handling**: A fully customized `thiserror` taxonomy ensures that internal anomalies safely map into structured `IntoResponse` protocol payloads automatically.

## Getting Started

### Prerequisites

- [Rust Toolchain (1.70+)](https://rustup.rs/)
- [Docker Engine & Docker Compose](https://docs.docker.com/engine/install/) (for localized testing nodes)

### Spin up Local Dependencies

The project relies on SurrealDB for persistence and Prometheus for visualizing telemetry.

Start the development cluster via Docker:

```bash
docker-compose up -d
```

This launches:
- SurrealDB on port `8000` (in-memory mode)
- Prometheus on port `9090` (scrapes `:9678/metrics` every 5 s)

### Configuration

The service is configured entirely via environment variables. Copy `.env.local` and adjust as needed.

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `127.0.0.1` | Network interface to bind to. |
| `PORT` | `9678` / `8443` | Listen port (`9678` for HTTP, `8443` for HTTPS). |
| `ENABLE_TLS` | `false` | Toggle auto-generated self-signed TLS via `rcgen`. |
| `TLS_ALT_NAME` | *(empty)* | Extra Subject Alternative Name (SAN) in the generated cert. |
| `SURREALDB_URL` | `ws://127.0.0.1:8000` | SurrealDB connection URL (`ws://` or `http://`). |
| `SURREALDB_USER` | `root` | SurrealDB root username. |
| `SURREALDB_PASS` | `root` | SurrealDB root password. |
| `SURREALDB_NS` | `merk` | SurrealDB namespace. |
| `SURREALDB_DB` | `merk` | SurrealDB database name. |
| `JWT_SECRET` | *(dev default)* | JWT signing secret — **must be ≥ 32 characters**. The default is rejected in `--release` builds. |

### Run the Application

```bash
cargo run
```

The terminal will display an ASCII banner with the active bind address and TLS state.

## Endpoints

### REST — `/api/v1`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/health` | — | Service health check with DB latency. |
| `GET` | `/api/v1/error` | — | Returns a sample structured error (debug/demo). |
| `POST` | `/api/v1/auth/register` | — | Register a new user; returns JWT + user object. |
| `POST` | `/api/v1/auth/login` | — | Login; returns JWT + user object. |
| `POST` | `/api/v1/auth/logout` | Bearer JWT | Stateless logout (returns 204). |
| `POST` | `/api/v1/auth/reset-password` | Bearer JWT | Change password using old + new password. |
| `PUT` | `/api/v1/auth/{id}/deactivate` | Bearer JWT | Deactivate own account (users can only deactivate themselves). |
| `GET` | `/api/v1/auth/me` | Bearer JWT | Fetch current user profile. |
| `GET` | `/metrics` | — | Prometheus metrics endpoint. |

### GraphQL — `/graphql`

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/graphql` | GraphiQL browser UI. |
| `POST` | `/graphql` | GraphQL executor. |

**Queries**
- `me` — returns the authenticated user (requires `Authorization: Bearer <token>` header).

**Mutations**
- `registerUser(username, email, password)` → `AuthPayload`
- `loginUser(email, password)` → `AuthPayload`
- `logoutUser` → `Boolean`
- `resetPassword(oldPassword, newPassword)` → `Boolean`
- `deactivateUser(userId)` → `Boolean`

`AuthPayload` contains `{ token, user: { id, username, email, is_active, is_verified } }`.

### Interactive Explorers

| Tool | URL |
|------|-----|
| OpenAPI / Scalar | [http://127.0.0.1:9678/docs/scalar](http://127.0.0.1:9678/docs/scalar) |
| OpenAPI JSON | [http://127.0.0.1:9678/docs/openapi.json](http://127.0.0.1:9678/docs/openapi.json) |
| GraphQL / GraphiQL | [http://127.0.0.1:9678/graphql](http://127.0.0.1:9678/graphql) |

*(Prefix with `https://` when `ENABLE_TLS=true`.)*

## Database Schema

Migrations are embedded in the binary via `rust-embed` and run automatically on startup. Applied migrations are tracked in a `_migrations` table.

### Migration 0001 — Initial Schema

| Table | Key Fields |
|-------|-----------|
| `user` | `username` (unique), `email` (unique), `password_hash`, `is_active`, `is_verified`, `created_at`, `updated_at`, `last_login` |
| `profile` | `user` (record link), `first_name`, `last_name`, `display_name`, `avatar_url`, `bio`, `language`, `country`, `timezone` |

### Migration 0002 — RBAC Graphs

SurrealDB graph edges power the permission system:

```
user --[assigned_role]--> role --[has_permission]--> permission
```

Seeded roles: `admin`, `user`.  
Seeded permissions: `manage_users`, `read_content`, `write_content`.

> **Note:** RBAC graph queries are implemented in `src/db/rbac_repo.rs` but no HTTP endpoints for role assignment are wired up yet — this is an intentional extension point.

## Authentication

- Tokens are **HS256 JWTs** signed with `JWT_SECRET`, valid for **24 hours**.
- Pass tokens in the `Authorization: Bearer <token>` header.
- Logout is client-side only — the server does not maintain a token blocklist.
- Suspended users (`is_active = false`) receive a `403 Forbidden` on login and on every authenticated request.

## Testing

Unit and integration tests run against an in-memory SurrealDB instance — no external services required:

```bash
cargo test
```

A Criterion benchmark for the Argon2 hashing layer is available under `benches/`:

```bash
cargo bench
```

## Deployment

### Docker

```bash
docker-compose up --build
```

### Kubernetes (Timoni)

The `timoni/` directory contains a [Timoni](https://timoni.sh/) bundle (CUE-based) that generates a full Kubernetes deployment:

- `Deployment` for `merk` with liveness/readiness probes on `/api/v1/health`
- Co-deployed in-memory SurrealDB `Deployment` + `Service`
- `ConfigMap` for non-secret env vars
- `Secret` for `JWT_SECRET` and `SURREALDB_PASS`

Default image: `ghcr.io/musanif-e-rekhta/merk:latest`.  
Resource defaults: 100m CPU / 128 Mi RAM; limits 500m / 512 Mi.

### CI

GitHub Actions (`.github/workflows/ci.yml`) runs on every push:
1. `cargo fmt --check`
2. `cargo clippy`
3. `cargo test --all-features`
4. `cargo build --release` (artifact upload)

## Architecture

```
src/
  main.rs           — binary entry point, telemetry init, env loading
  lib.rs            — re-exports all public modules
  server.rs         — server startup, TLS, Prometheus, graceful shutdown
  config.rs         — AppConfig (envy env → typed struct)
  state.rs          — AppState (Arc<AppConfig> + Db, injected via Axum State)
  error.rs          — thiserror error taxonomy, IntoResponse, ErrorResponse
  api/
    mod.rs          — create_router(): merges sub-routers, attaches TraceLayer
    v1/
      health.rs     — GET /health, GET /error
      users.rs      — auth REST handlers
    graphql/        — async-graphql schema + axum integration
    openapi/        — aide OpenAPI spec, Scalar UI
    middleware/     — Claims extractor (JWT Bearer validation)
  db/
    mod.rs          — Db type alias, connect_to_db(), migration runner
    user_repo.rs    — UserRepo CRUD + auth ops
    profile_repo.rs — ProfileRepo
    rbac_repo.rs    — RbacRepo (graph-based permission checks)
    migrations/     — .surql migration files embedded at compile time
  services/
    auth.rs         — hash_password(), verify_password(), generate_jwt()
```

**Key design decisions:**
- `AppState` is cloned cheaply — `AppConfig` is behind `Arc`, `Db` is an internal `Arc`-backed handle.
- Errors never leak internal details to clients; `Internal` and `Upstream` variants are logged server-side and surface only as `500`/`502` with opaque messages.
- All DB integration tests use `mem://` SurrealDB so CI never needs a running database.

## License

This project relies on a CC0 1.0 Universal license. Please refer to `LICENSE` for more information.
