#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    DuplicateId,
    BrokenReference,
    MalformedFragment,
    InvalidId,
    InvalidEdgeEndpoint,
    InvalidPath,
    UnknownNodeKind,
    UnknownEdgeKind,
    ScenarioMissingCommand,
    ScenarioMissingCrate,
    ArtifactMissingProducer,
    CommandReferencedButUndeclared,
    InvalidConfig,
    DiscoveryFailure,
    QueryRootMissing,
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateId => "duplicate_id",
            Self::BrokenReference => "broken_reference",
            Self::MalformedFragment => "malformed_fragment",
            Self::InvalidId => "invalid_id",
            Self::InvalidEdgeEndpoint => "invalid_edge_endpoint",
            Self::InvalidPath => "invalid_path",
            Self::UnknownNodeKind => "unknown_node_kind",
            Self::UnknownEdgeKind => "unknown_edge_kind",
            Self::ScenarioMissingCommand => "scenario_missing_command",
            Self::ScenarioMissingCrate => "scenario_missing_crate",
            Self::ArtifactMissingProducer => "artifact_missing_producer",
            Self::CommandReferencedButUndeclared => "command_referenced_but_undeclared",
            Self::InvalidConfig => "invalid_config",
            Self::DiscoveryFailure => "discovery_failure",
            Self::QueryRootMissing => "query_root_missing",
        }
    }

    pub fn default_message(self) -> &'static str {
        match self {
            Self::DuplicateId => "duplicate atlas id",
            Self::BrokenReference => "broken graph reference",
            Self::MalformedFragment => "malformed atlas fragment",
            Self::InvalidId => "invalid atlas id",
            Self::InvalidEdgeEndpoint => "invalid edge endpoint kinds",
            Self::InvalidPath => "invalid source path",
            Self::UnknownNodeKind => "unknown node kind",
            Self::UnknownEdgeKind => "unknown edge kind",
            Self::ScenarioMissingCommand => "scenario is missing a proving command",
            Self::ScenarioMissingCrate => "scenario is missing an exercised crate",
            Self::ArtifactMissingProducer => "artifact is missing a producer edge",
            Self::CommandReferencedButUndeclared => "command was referenced but not declared",
            Self::InvalidConfig => "invalid atlas configuration",
            Self::DiscoveryFailure => "discovery failed",
            Self::QueryRootMissing => "requested trace root is missing",
        }
    }

    pub fn default_severity(self) -> Severity {
        match self {
            Self::InvalidPath => Severity::Warning,
            Self::ArtifactMissingProducer => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    Usage = 2,
    ValidationFailed = 3,
    RuntimeError = 4,
}
