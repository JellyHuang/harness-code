//! Hook system for event-driven extensibility.

mod events;
mod registry;
mod executor;

pub use events::*;
pub use registry::*;
pub use executor::*;