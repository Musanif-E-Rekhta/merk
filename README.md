# merk

A blazing-fast, foundational Rust web service built with [Axum](https://github.com/tokio-rs/axum).

This project establishes an enterprise-grade architecture out of the box, equipped with REST protocols, GraphQL interfaces, API documentation generation, metrics extraction, and robust typed error handling.

## Features

- **Robust Framework**: Powered by Tokio and Axum for high-performance asynchronous execution.
- **REST & GraphQL Dual Support**: First-class handling of traditional REST routes alongside `async-graphql` schemas.
- **Auto-Generating OpenAPI Docs**: Routes mapped using `aide` automatically reflect in a beautiful scalar-driven UI (equipped with pre-configured JWT Bearer authentication).
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
*(This launches an empty SurrealDB node on `8000` and Prometheus on `9090` locally).*

### Configuration Details

The web service can be configured via environment variables.

| Environment Variable | Default value | Description |
| -------------------- | ------------- | ----------- |
| `HOST`               | `127.0.0.1`   | The network interface to bind the application to. |
| `PORT`               | `9678` / `8443`| Explicit API port layout (`9678` if HTTP, `8443` if HTTPS). |
| `ENABLE_TLS`         | `false`       | Toggles the internal `axum-server` to boot via an auto-generated TLS certificate layout. |
| `TLS_ALT_NAME`       | *(empty)*     | Specify an alternative Subject Alternative Name (SAN) when rendering the development cert override. |

### Run the Application

Compile and launch the server using Cargo:

```bash
cargo run
```
You will be greeted by a terminal ASCII banner indicating active bindings and operational states!

## Endpoints

Once running, the core topology exposes:

- **`GET /api/v1/health`**: Instant REST health check.
- **`GET /api/v1/error`**: Debug sandbox routing for structural taxonomy error parsing.
- **`GET /graphql`**: Fully interactive browser-ready GraphiQL UI testing explorer.
- **`POST /graphql`**: Live async-graphql executor connection.
- **`GET /metrics`**: Prometheus telemetry metrics.

### Interactive Explorers

Rather than manually managing Postman collections, the server natively hosts its own visual testing environments:

- **OpenAPI / Scalar**: [http://127.0.0.1:9678/docs/scalar](http://127.0.0.1:9678/docs/scalar)
- **GraphQL / GraphiQL**: [http://127.0.0.1:9678/graphql](http://127.0.0.1:9678/graphql)

*(Ensure that `ENABLE_TLS` is toggled off, or connect securely via `https://`)*

## Architecture Pattern

This project enforces a highly scalable codebase layout using the domain-driven `axum::extract::State` protocol natively:
- **`src/main.rs`**: Bootstraps the application binary footprint and telemetry logic gracefully.
- **`src/lib.rs`**: Isolates internal modules safely allowing complex integration test suites to construct the API seamlessly across mock environments.
- **`src/api/*`**: Organizes transport layouts securely. It divides protocols into specific `openapi`, `rest`, and `graphql` sub-handlers intuitively.
- **`src/state.rs`**: Centralizes memory allocations natively inside an `Arc` structure. Use it to pass heavy configurations or active `.db` client channels directly downward to isolated API handler instances without resorting to globals.

## License

This project relies on a CC0 1.0 Universal license. Please refer to `LICENSE` for more information.
