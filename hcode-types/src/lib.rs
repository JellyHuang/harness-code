//! HCode Types - Core domain types for the HCode agent system.
//!
//! This crate provides the foundational types used across all HCode components:
//! - Messages and content blocks for LLM communication
//! - Tool definitions and results
//! - Task notifications for agent coordination
//! - Permission types for access control
//! - Error types for the application

pub mod error;
pub mod message;
pub mod notification;
pub mod permission;
pub mod tool;
pub mod usage;

pub use error::*;
pub use message::*;
pub use notification::*;
pub use permission::*;
pub use tool::*;
pub use usage::*;

// Re-export Usage from message module explicitly to avoid ambiguity with notification::Usage
pub use message::Usage;
