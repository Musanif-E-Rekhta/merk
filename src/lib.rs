//! merk — axum + SurrealDB backend for the Musanif Urdu reading platform.
//!
//! The HTTP surface is GraphQL-first: every authenticated, admin, and
//! composite operation lives in [`api::graphql`]. The only REST routes
//! are `/health`, `/metrics`, `/docs`, and a small admin slice for
//! multipart upload + binary page previews — see [`api::v1`].
//!
//! Composition is handled in [`server::start`] (env loading + tracing
//! init) and [`api::create_router`] (REST + GraphQL + metrics + SPA).
//! The library is exposed mostly for integration tests; production
//! callers go through the `merk` binary.
//!
//! Sibling crates do the heavy lifting: see `README.md` for the full
//! list (`merk-auth`, `merk-totp`, `merk-rbac`, `merk-blob-store`,
//! `merk-events`, `merk-ingest`, `merk-observability`,
//! `merk-axum-middleware`, `merk-migrations`).

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod server;
pub mod services;
pub mod state;
pub mod utils;

#[doc(hidden)]
pub use anyhow;
