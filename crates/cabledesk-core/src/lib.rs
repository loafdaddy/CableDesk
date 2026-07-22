//! Shared types used across every CableDesk component: the connection state
//! machine, the error model, and the on-disk configuration shapes.
//!
//! This crate must stay platform-independent. Anything Fedora-specific
//! belongs in `cabledesk-platform-fedora`.

pub mod config;
pub mod device;
pub mod error;
pub mod state;

pub use config::{DeviceConfig, Role};
pub use device::{DeviceId, PairingCode, TrustedDevice};
pub use error::{CableDeskError, Result, UserFacingError};
pub use state::{ConnectionState, StateMachine, TransitionError};
