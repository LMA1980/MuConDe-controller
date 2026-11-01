//! Continuous delivery control plane orchestrator for the MuConDe distributed deployment fabric.
//!
//! Provides zero-downtime A/B rollouts, cluster idle worker discovery, TTL-bounded deployment leases,
//! declarative failure contracts, atomic slot promotion, automatic rollback, and registry digest tainting.

#![deny(unsafe_code)]

pub mod contract;
pub mod error;
pub mod lease;
pub mod orchestrator;
pub mod tls;

// Re-exports of common primary types for ergonomic library use
pub use contract::{
    IntegrationTestSelector, MuConDeConfig, DEFAULT_LEASE_TTL_SECONDS,
    DEFAULT_WARMUP_TIMEOUT_SECONDS,
};
pub use error::{ContractError, ControllerError, LeaseError, OrchestratorError, TlsError};
pub use lease::{ClusterIdlePoller, ClusterNodeStatus, LeaseCoordinator, DEFAULT_CONTROLLER_URL};
pub use orchestrator::{
    AgentIpcClient, DeploymentOrchestrator, DeploymentOutcome, HttpMuRegistClient, IpcAgentClient,
    MuRegistClient, DEFAULT_REGISTRY_URL,
};
pub use tls::TlsConfig;

/// Daemon version string.
pub const CONTROLLER_VERSION: &str = env!("CARGO_PKG_VERSION");
