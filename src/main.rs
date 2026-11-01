//! Daemon and CLI entry point for MuConDe-controller.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use muconde_controller::contract::MuConDeConfig;
use muconde_controller::lease::{ClusterIdlePoller, LeaseCoordinator, DEFAULT_CONTROLLER_URL};
use muconde_controller::orchestrator::{
    DeploymentOrchestrator, DeploymentOutcome, HttpMuRegistClient, IpcAgentClient,
    DEFAULT_REGISTRY_URL,
};
use muconde_controller::tls::TlsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!(
        version = muconde_controller::CONTROLLER_VERSION,
        "Starting MuConDe-controller CD control plane"
    );

    let controller_url = std::env::var("MUCONDE_CONTROLLER_URL")
        .unwrap_or_else(|_| DEFAULT_CONTROLLER_URL.to_string());
    let regist_url =
        std::env::var("MUREGIST_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string());
    let agent_port: u16 = std::env::var("MUCONDE_AGENT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8446);
    let auth_token = std::env::var("MUREGIST_TOKEN").ok();

    let ca_path = std::env::var("TLS_CA_CERT").ok().map(PathBuf::from);
    let client_cert_path = std::env::var("TLS_CLIENT_CERT").ok().map(PathBuf::from);
    let client_key_path = std::env::var("TLS_CLIENT_KEY").ok().map(PathBuf::from);

    let tls_config = TlsConfig {
        ca_cert_path: ca_path,
        client_cert_path,
        client_key_path,
        ..Default::default()
    };

    let coordinator = Arc::new(LeaseCoordinator::new());
    let poller = match ClusterIdlePoller::new(&controller_url, &tls_config) {
        Ok(p) => Some(p),
        Err(e) => {
            error!(error = %e, "Failed to initialize cluster poller; proceeding with poller disabled");
            None
        }
    };

    let regist_client = Arc::new(HttpMuRegistClient::new(
        &regist_url,
        &tls_config,
        auth_token,
    )?);
    let agent_client = Arc::new(IpcAgentClient::new(agent_port));

    let orchestrator = Arc::new(DeploymentOrchestrator::new(
        coordinator.clone(),
        poller,
        agent_client,
        regist_client,
    ));

    // Spawn background lease expiration watcher (sweeping every 1s)
    let (revoke_tx, mut revoke_rx) = tokio::sync::mpsc::channel(64);
    let _watcher_handle = coordinator
        .clone()
        .spawn_expiration_watcher(Duration::from_secs(1), revoke_tx);

    tokio::spawn(async move {
        while let Some((revoke, rollback)) = revoke_rx.recv().await {
            info!(
                deployment_id = %revoke.deployment_id,
                lease_id = %revoke.lease_id,
                service = %rollback.service_name,
                reason = %rollback.reason,
                "Background sweep evicted expired deployment lease"
            );
        }
    });

    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "deploy" {
        let config_path = &args[2];
        let artifact_sha256 = args
            .get(3)
            .map(|s| s.as_str())
            .unwrap_or("unspecified-digest");

        info!(
            config = %config_path,
            digest = %artifact_sha256,
            "Executing standalone deployment dispatch"
        );

        let config = MuConDeConfig::from_file(config_path)?;
        let outcome = orchestrator
            .orchestrate_deployment(&config, artifact_sha256, None)
            .await?;

        match outcome {
            DeploymentOutcome::Promoted {
                service,
                target_node,
                artifact_sha256,
                ..
            } => {
                info!(
                    service = %service,
                    node = %target_node,
                    digest = %artifact_sha256,
                    "Deployment successfully completed and promoted"
                );
            }
            DeploymentOutcome::RolledBack {
                service,
                target_node,
                artifact_sha256,
                reason,
                ..
            } => {
                error!(
                    service = %service,
                    node = %target_node,
                    digest = %artifact_sha256,
                    reason = %reason,
                    "Deployment failed and was rolled back"
                );
                std::process::exit(1);
            }
        }

        return Ok(());
    }

    info!("MuConDe-controller daemon running; waiting for shutdown signal");
    tokio::signal::ctrl_c().await?;
    info!("Received termination signal; shutting down");

    Ok(())
}
