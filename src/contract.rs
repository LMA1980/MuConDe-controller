//! Declarative continuous delivery deployment contract (`.muconde/MuConDe.yml`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ContractError;

/// Default lease time-to-live in seconds (300 seconds / 5 minutes).
pub const DEFAULT_LEASE_TTL_SECONDS: u64 = 300;

/// Default post-swap warmup and health probe timeout in seconds (30 seconds).
pub const DEFAULT_WARMUP_TIMEOUT_SECONDS: u64 = 30;

/// Default canary bake SLO evaluation window in seconds (60 seconds).
pub const DEFAULT_BAKE_WINDOW_SECONDS: u64 = 60;

/// Default maximum error log count permitted during a canary bake window (0).
pub const DEFAULT_MAX_ERROR_LOGS: u32 = 0;

/// Default maximum p99 latency in milliseconds permitted during a canary bake window (250.0 ms).
pub const DEFAULT_MAX_P99_LATENCY_MS: f64 = 250.0;

fn default_lease_ttl() -> u64 {
    DEFAULT_LEASE_TTL_SECONDS
}

fn default_warmup_timeout() -> u64 {
    DEFAULT_WARMUP_TIMEOUT_SECONDS
}

fn default_bake_window() -> u64 {
    DEFAULT_BAKE_WINDOW_SECONDS
}

fn default_max_error_logs() -> u32 {
    DEFAULT_MAX_ERROR_LOGS
}

fn default_max_p99_latency_ms() -> f64 {
    DEFAULT_MAX_P99_LATENCY_MS
}

/// Declarative continuous delivery configuration loaded from `.muconde/MuConDe.yml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MuConDeConfig {
    pub version: String,
    pub service: String,
    pub target_nodes: Vec<String>,
    #[serde(default = "default_lease_ttl")]
    pub lease_ttl_seconds: u64,
    #[serde(default = "default_warmup_timeout")]
    pub warmup_timeout_seconds: u64,
    #[serde(default = "default_bake_window")]
    pub bake_window_seconds: u64,
    #[serde(default = "default_max_error_logs")]
    pub max_error_logs: u32,
    #[serde(default = "default_max_p99_latency_ms")]
    pub max_p99_latency_ms: f64,
    pub integration_tests: IntegrationTestSelector,
}

/// Selector specifying which integration test suites are required before promotion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum IntegrationTestSelector {
    /// Target all integration test suites (`"all"`).
    All(String),
    /// Target an explicit list of integration test identifiers.
    Selected(Vec<String>),
}

impl MuConDeConfig {
    /// Parse and validate configuration from a YAML string.
    pub fn from_yaml_str(yaml_str: &str) -> Result<Self, ContractError> {
        let config: Self = serde_yaml::from_str(yaml_str)?;
        config.validate()?;
        Ok(config)
    }

    /// Load and validate configuration from a file on disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ContractError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ContractError::Yaml(e.to_string()))?;
        Self::from_yaml_str(&content)
    }

    /// Validate all contract fields against CD architectural invariants.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.version != "1" {
            return Err(ContractError::UnsupportedVersion(self.version.clone()));
        }

        if self.service.trim().is_empty() {
            return Err(ContractError::EmptyService);
        }

        if self.target_nodes.is_empty() {
            return Err(ContractError::EmptyTargetNodes);
        }

        for node in &self.target_nodes {
            if node.trim().is_empty() {
                return Err(ContractError::EmptyTargetNode);
            }
        }

        if self.lease_ttl_seconds == 0 {
            return Err(ContractError::InvalidLeaseTtl);
        }

        if self.warmup_timeout_seconds == 0 {
            return Err(ContractError::InvalidWarmupTimeout);
        }

        match &self.integration_tests {
            IntegrationTestSelector::All(s) => {
                if !s.trim().eq_ignore_ascii_case("all") {
                    return Err(ContractError::InvalidSelector(s.clone()));
                }
            }
            IntegrationTestSelector::Selected(list) => {
                if list.is_empty() {
                    return Err(ContractError::EmptyTestList);
                }
                for test in list {
                    if test.trim().is_empty() {
                        return Err(ContractError::EmptyTestName);
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns `true` if all integration tests should be executed.
    pub fn should_run_all(&self) -> bool {
        matches!(&self.integration_tests, IntegrationTestSelector::All(_))
    }

    /// Returns a slice of explicitly selected test names, or `None` if `"all"` was selected.
    pub fn selected_tests(&self) -> Option<&[String]> {
        match &self.integration_tests {
            IntegrationTestSelector::All(_) => None,
            IntegrationTestSelector::Selected(tests) => Some(tests.as_slice()),
        }
    }

    /// Checks if a given test name should be executed according to this configuration.
    pub fn matches_test(&self, test_name: &str) -> bool {
        match &self.integration_tests {
            IntegrationTestSelector::All(_) => true,
            IntegrationTestSelector::Selected(tests) => tests.iter().any(|t| t == test_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_all_selector() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes:
  - "node-a"
  - "worker-riscv64"
lease_ttl_seconds: 180
warmup_timeout_seconds: 25
integration_tests: "all"
"#;
        let config = MuConDeConfig::from_yaml_str(yaml).expect("valid config with all tests");
        assert_eq!(config.version, "1");
        assert_eq!(config.service, "action-runner");
        assert_eq!(config.target_nodes, vec!["node-a", "worker-riscv64"]);
        assert_eq!(config.lease_ttl_seconds, 180);
        assert_eq!(config.warmup_timeout_seconds, 25);
        assert_eq!(
            config.integration_tests,
            IntegrationTestSelector::All("all".to_string())
        );
        assert!(config.should_run_all());
        assert!(config.matches_test("any_test"));
        assert_eq!(config.selected_tests(), None);
    }

    #[test]
    fn test_parse_valid_selected_selector_with_defaults() {
        let yaml = r#"
version: "1"
service: "muconde-agent"
target_nodes:
  - "node-a"
integration_tests:
  - "it_smoke"
  - "it_mtls_handshake"
"#;
        let config = MuConDeConfig::from_yaml_str(yaml).expect("valid config with selected tests");
        assert_eq!(config.version, "1");
        assert_eq!(config.service, "muconde-agent");
        assert_eq!(config.lease_ttl_seconds, DEFAULT_LEASE_TTL_SECONDS);
        assert_eq!(
            config.warmup_timeout_seconds,
            DEFAULT_WARMUP_TIMEOUT_SECONDS
        );
        assert_eq!(
            config.integration_tests,
            IntegrationTestSelector::Selected(vec![
                "it_smoke".to_string(),
                "it_mtls_handshake".to_string(),
            ])
        );
        assert!(!config.should_run_all());
        assert!(config.matches_test("it_smoke"));
        assert!(config.matches_test("it_mtls_handshake"));
        assert!(!config.matches_test("it_unknown"));
        assert_eq!(
            config.selected_tests(),
            Some(&["it_smoke".to_string(), "it_mtls_handshake".to_string()][..])
        );
    }

    #[test]
    fn test_reject_unsupported_version() {
        let yaml = r#"
version: "2"
service: "action-runner"
target_nodes: ["node-a"]
integration_tests: "all"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::UnsupportedVersion("2".to_string()));
    }

    #[test]
    fn test_reject_empty_service() {
        let yaml = r#"
version: "1"
service: "   "
target_nodes: ["node-a"]
integration_tests: "all"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::EmptyService);
    }

    #[test]
    fn test_reject_empty_target_nodes() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: []
integration_tests: "all"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::EmptyTargetNodes);
    }

    #[test]
    fn test_reject_empty_target_node_name() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: ["node-a", "  "]
integration_tests: "all"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::EmptyTargetNode);
    }

    #[test]
    fn test_reject_zero_lease_ttl() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: ["node-a"]
lease_ttl_seconds: 0
integration_tests: "all"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::InvalidLeaseTtl);
    }

    #[test]
    fn test_reject_zero_warmup_timeout() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: ["node-a"]
warmup_timeout_seconds: 0
integration_tests: "all"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::InvalidWarmupTimeout);
    }

    #[test]
    fn test_reject_invalid_string_selector() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: ["node-a"]
integration_tests: "none"
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::InvalidSelector("none".to_string()));
    }

    #[test]
    fn test_reject_empty_selected_test_list() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: ["node-a"]
integration_tests: []
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::EmptyTestList);
    }

    #[test]
    fn test_reject_empty_test_name_in_selected_list() {
        let yaml = r#"
version: "1"
service: "action-runner"
target_nodes: ["node-a"]
integration_tests:
  - "it_smoke"
  - "   "
"#;
        let err = MuConDeConfig::from_yaml_str(yaml).unwrap_err();
        assert_eq!(err, ContractError::EmptyTestName);
    }
}
