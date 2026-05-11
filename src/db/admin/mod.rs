//! Admin pipeline repos. Each module wraps one or two tables from
//! migrations 0010–0019 plus the related transitions.
//!
//! Mutations that materially change observable state (job status, draft
//! decisions) emit events through `services::event_bus::EventBus` so the
//! GraphQL Subscription root can stream them out.

pub mod ai;
pub mod covers;
pub mod drafts;
pub mod jobs;
pub mod publish;
