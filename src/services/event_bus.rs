//! Two-bus container for the merk admin pipeline.
//!
//! Wraps two `merk_events::EventBus<E>` instances — one for `JobEvent`s
//! (high-volume step/log/completed traffic) and one for `DraftEvent`s
//! (lower-volume review-stage transitions). Keeps slow draft-event
//! subscribers from starving job-log traffic on the same broadcast channel.
//!
//! Re-exports `JobEvent` and `DraftEvent` from `merk_events` so consumers
//! can keep `use crate::services::event_bus::{JobEvent, DraftEvent}`.

use merk_events::EventBus as GenericBus;
use std::sync::Arc;

pub use merk_events::{DraftEvent, JobEvent};

#[derive(Clone)]
pub struct EventBus {
    pub jobs: Arc<GenericBus<JobEvent>>,
    pub drafts: Arc<GenericBus<DraftEvent>>,
}

impl EventBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs: GenericBus::new(),
            drafts: GenericBus::new(),
        })
    }

    pub fn publish_job(&self, ev: JobEvent) {
        self.jobs.publish(ev);
    }

    pub fn publish_draft(&self, ev: DraftEvent) {
        self.drafts.publish(ev);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self {
            jobs: GenericBus::new(),
            drafts: GenericBus::new(),
        }
    }
}
