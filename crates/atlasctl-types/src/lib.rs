#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

pub const ATLAS_SCHEMA_VERSION: u32 = 1;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
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
    DeadSelector,
    OrphanNode,
    StaleCommand,
    BrokenDocLink,
    DuplicateOwnership,
    EmptyFragment,
    RequirementNotProven,
    UncoveredCrate,
    ActiveGoalMissingPlan,
    ActiveGoalMissingReadyWorkItems,
    ActiveGoalWorkItemMissingProof,
    ClaimMissingProofCommand,
    PolicyLedgerMissingProofCommand,
    PolicyFileLegacyNoAtlas,
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
            Self::DeadSelector => "dead_selector",
            Self::OrphanNode => "orphan_node",
            Self::StaleCommand => "stale_command",
            Self::BrokenDocLink => "broken_doc_link",
            Self::DuplicateOwnership => "duplicate_ownership",
            Self::EmptyFragment => "empty_fragment",
            Self::RequirementNotProven => "requirement_not_proven",
            Self::UncoveredCrate => "uncovered_crate",
            Self::ActiveGoalMissingPlan => "active_goal_missing_plan",
            Self::ActiveGoalMissingReadyWorkItems => "active_goal_missing_ready_work_items",
            Self::ActiveGoalWorkItemMissingProof => "active_goal_work_item_missing_proof",
            Self::ClaimMissingProofCommand => "claim_missing_proof_command",
            Self::PolicyLedgerMissingProofCommand => "policy_ledger_missing_proof_command",
            Self::PolicyFileLegacyNoAtlas => "policy_file_legacy_no_atlas",
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
            Self::DeadSelector => "path selector matches no files",
            Self::OrphanNode => "node has no relationships",
            Self::StaleCommand => "command is not exercised by any scenario",
            Self::BrokenDocLink => "documentation link is broken",
            Self::DuplicateOwnership => "path is claimed by multiple nodes",
            Self::EmptyFragment => "fragment file contains no atlas metadata",
            Self::RequirementNotProven => "requirement is not proven by any scenario",
            Self::UncoveredCrate => "crate is not exercised by any scenario",
            Self::ActiveGoalMissingPlan => "active goal references a missing plan",
            Self::ActiveGoalMissingReadyWorkItems => "active goal has no ready work items",
            Self::ActiveGoalWorkItemMissingProof => "active goal work item has no proof command",
            Self::ClaimMissingProofCommand => "claim is missing a proof command",
            Self::PolicyLedgerMissingProofCommand => "policy_ledger is missing a proof command",
            Self::PolicyFileLegacyNoAtlas => "policy file has no atlas metadata and was skipped",
        }
    }

    pub fn default_severity(self) -> Severity {
        match self {
            Self::InvalidPath => Severity::Warning,
            Self::ArtifactMissingProducer => Severity::Warning,
            Self::DeadSelector => Severity::Warning,
            Self::OrphanNode => Severity::Warning,
            Self::StaleCommand => Severity::Warning,
            Self::EmptyFragment => Severity::Info,
            Self::RequirementNotProven => Severity::Warning,
            Self::UncoveredCrate => Severity::Warning,
            Self::ActiveGoalMissingReadyWorkItems => Severity::Warning,
            Self::ActiveGoalWorkItemMissingProof => Severity::Warning,
            Self::ClaimMissingProofCommand => Severity::Warning,
            Self::PolicyLedgerMissingProofCommand => Severity::Warning,
            Self::PolicyFileLegacyNoAtlas => Severity::Warning,
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

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct AtlasId(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AtlasIdError {
    #[error("atlas ids must follow <kind>:<slug>, got `{input}`")]
    InvalidFormat { input: String },
    #[error("atlas id kind must be lowercase ASCII with digits or underscores, got `{kind}`")]
    InvalidKind { kind: String },
    #[error(
        "atlas id slug must be lowercase ASCII with digits, dashes, or underscores, got `{slug}`"
    )]
    InvalidSlug { slug: String },
}

impl AtlasId {
    pub fn parse(input: impl Into<String>) -> Result<Self, AtlasIdError> {
        let input = input.into();
        let (kind, slug) = input
            .split_once(':')
            .ok_or_else(|| AtlasIdError::InvalidFormat {
                input: input.clone(),
            })?;

        if !kind_is_valid(kind) {
            return Err(AtlasIdError::InvalidKind {
                kind: kind.to_string(),
            });
        }

        if !slug_is_valid(slug) {
            return Err(AtlasIdError::InvalidSlug {
                slug: slug.to_string(),
            });
        }

        Ok(Self(input))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn kind_prefix(&self) -> &str {
        self.0
            .split_once(':')
            .map(|(kind, _)| kind)
            .unwrap_or_default()
    }

    pub fn slug(&self) -> &str {
        self.0
            .split_once(':')
            .map(|(_, slug)| slug)
            .unwrap_or_default()
    }
}

impl fmt::Display for AtlasId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for AtlasId {
    type Err = AtlasIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s.to_string())
    }
}

fn kind_is_valid(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn slug_is_valid(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Behavior,
    Proof,
    Document,
    Artifact,
    Command,
    Infra,
}

impl NodeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Behavior => "behavior",
            Self::Proof => "proof",
            Self::Document => "document",
            Self::Artifact => "artifact",
            Self::Command => "command",
            Self::Infra => "infra",
        }
    }
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Requirement,
    Roadmap,
    Proposal,
    Spec,
    Adr,
    Plan,
    Goal,
    SupportTier,
    PolicyLedger,
    Closeout,
    Claim,
    Guide,
    Scenario,
    Fixture,
    Command,
    Artifact,
    Crate,
    Document,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Roadmap => "roadmap",
            Self::Proposal => "proposal",
            Self::Spec => "spec",
            Self::Adr => "adr",
            Self::Plan => "plan",
            Self::Goal => "goal",
            Self::SupportTier => "support_tier",
            Self::PolicyLedger => "policy_ledger",
            Self::Closeout => "closeout",
            Self::Claim => "claim",
            Self::Guide => "guide",
            Self::Scenario => "scenario",
            Self::Fixture => "fixture",
            Self::Command => "command",
            Self::Artifact => "artifact",
            Self::Crate => "crate",
            Self::Document => "document",
        }
    }

    pub fn role(self) -> NodeRole {
        match self {
            Self::Requirement => NodeRole::Behavior,
            Self::Roadmap => NodeRole::Document,
            Self::Proposal => NodeRole::Document,
            Self::Spec => NodeRole::Document,
            Self::Adr => NodeRole::Document,
            Self::Plan => NodeRole::Document,
            Self::Goal => NodeRole::Document,
            Self::SupportTier => NodeRole::Document,
            Self::PolicyLedger => NodeRole::Document,
            Self::Closeout => NodeRole::Document,
            Self::Claim => NodeRole::Document,
            Self::Guide => NodeRole::Document,
            Self::Scenario => NodeRole::Proof,
            Self::Fixture => NodeRole::Proof,
            Self::Command => NodeRole::Command,
            Self::Artifact => NodeRole::Artifact,
            Self::Crate => NodeRole::Infra,
            Self::Document => NodeRole::Document,
        }
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl FromStr for NodeKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "requirement" => Ok(Self::Requirement),
            "roadmap" => Ok(Self::Roadmap),
            "proposal" => Ok(Self::Proposal),
            "spec" => Ok(Self::Spec),
            "adr" => Ok(Self::Adr),
            "plan" => Ok(Self::Plan),
            "goal" => Ok(Self::Goal),
            "support_tier" => Ok(Self::SupportTier),
            "policy_ledger" => Ok(Self::PolicyLedger),
            "closeout" => Ok(Self::Closeout),
            "claim" => Ok(Self::Claim),
            "guide" => Ok(Self::Guide),
            "scenario" => Ok(Self::Scenario),
            "fixture" => Ok(Self::Fixture),
            "command" => Ok(Self::Command),
            "artifact" => Ok(Self::Artifact),
            "crate" => Ok(Self::Crate),
            "document" => Ok(Self::Document),
            other => Err(other.to_string()),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Explains,
    Proves,
    UsesFixture,
    RunsWith,
    Emits,
    Exercises,
    Documents,
    BelongsTo,
    Supports,
    Defines,
    Requires,
    Decides,
    Implements,
    ActiveFor,
    Claims,
    Governs,
    Closes,
    Supersedes,
    OwnsPath,
    TouchesPath,
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explains => "explains",
            Self::Proves => "proves",
            Self::UsesFixture => "uses_fixture",
            Self::RunsWith => "runs_with",
            Self::Emits => "emits",
            Self::Exercises => "exercises",
            Self::Documents => "documents",
            Self::BelongsTo => "belongs_to",
            Self::Supports => "supports",
            Self::Defines => "defines",
            Self::Requires => "requires",
            Self::Decides => "decides",
            Self::Implements => "implements",
            Self::ActiveFor => "active_for",
            Self::Claims => "claims",
            Self::Governs => "governs",
            Self::Closes => "closes",
            Self::Supersedes => "supersedes",
            Self::OwnsPath => "owns_path",
            Self::TouchesPath => "touches_path",
        }
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl FromStr for EdgeKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "explains" => Ok(Self::Explains),
            "proves" => Ok(Self::Proves),
            "uses_fixture" => Ok(Self::UsesFixture),
            "runs_with" => Ok(Self::RunsWith),
            "emits" => Ok(Self::Emits),
            "exercises" => Ok(Self::Exercises),
            "documents" => Ok(Self::Documents),
            "belongs_to" => Ok(Self::BelongsTo),
            "supports" => Ok(Self::Supports),
            "defines" => Ok(Self::Defines),
            "requires" => Ok(Self::Requires),
            "decides" => Ok(Self::Decides),
            "implements" => Ok(Self::Implements),
            "active_for" => Ok(Self::ActiveFor),
            "claims" => Ok(Self::Claims),
            "governs" => Ok(Self::Governs),
            "closes" => Ok(Self::Closes),
            "supersedes" => Ok(Self::Supersedes),
            "owns_path" => Ok(Self::OwnsPath),
            "touches_path" => Ok(Self::TouchesPath),
            other => Err(other.to_string()),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct RepoRelativePath(String);

impl RepoRelativePath {
    pub fn new(path: impl Into<String>) -> Self {
        let s = path.into();
        Self(s.replace('\\', "/"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepoRelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for RepoRelativePath {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for RepoRelativePath {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PathSelector {
    pub pattern: String,
}

impl PathSelector {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: RepoRelativePath::new(pattern).0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocation {
    pub path: RepoRelativePath,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Provenance {
    pub source: RepoRelativePath,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub fragment: Option<String>,
}

impl Provenance {
    pub fn new(source: RepoRelativePath) -> Self {
        Self {
            source,
            line: None,
            column: None,
            fragment: None,
        }
    }

    pub fn location(&self) -> SourceLocation {
        SourceLocation {
            path: self.source.clone(),
            line: self.line,
            column: self.column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AtlasNode {
    pub id: AtlasId,
    pub kind: NodeKind,
    pub role: NodeRole,
    pub title: String,
    pub summary: Option<String>,
    #[serde(default)]
    pub owns: Vec<PathSelector>,
    #[serde(default)]
    pub touches: Vec<PathSelector>,
    #[serde(default)]
    pub attrs: BTreeMap<String, Value>,
    pub provenance: Provenance,
}

impl AtlasNode {
    pub fn all_paths(&self) -> impl Iterator<Item = &PathSelector> {
        self.owns.iter().chain(self.touches.iter())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AtlasEdge {
    pub from: AtlasId,
    pub kind: EdgeKind,
    pub to: AtlasId,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AtlasDiagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub subject: Option<AtlasId>,
    pub location: Option<SourceLocation>,
}

impl AtlasDiagnostic {
    pub fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        subject: Option<AtlasId>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            code,
            severity: code.default_severity(),
            message: message.into(),
            subject,
            location,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AtlasMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    pub diagnostic_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoDescriptor {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AtlasGraph {
    pub schema_version: u32,
    pub tool_version: String,
    pub repo: RepoDescriptor,
    pub nodes: Vec<AtlasNode>,
    pub edges: Vec<AtlasEdge>,
    pub diagnostics: Vec<AtlasDiagnostic>,
    pub metrics: AtlasMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DiscoveredRepo {
    pub repo: RepoDescriptor,
    pub config: AtlasConfig,
    pub nodes: Vec<AtlasNode>,
    pub edges: Vec<AtlasEdge>,
    pub diagnostics: Vec<AtlasDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct ActiveGoalConfig {
    pub goal: Option<String>,
    pub plan: Option<String>,
    #[serde(default)]
    pub proposal: Option<String>,
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub ready_work_items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiscoveryConfig {
    #[serde(default = "default_roots")]
    pub roots: Vec<String>,
    #[serde(default = "default_ignored_paths")]
    pub ignore: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            roots: default_roots(),
            ignore: default_ignored_paths(),
        }
    }
}

fn default_roots() -> Vec<String> {
    vec![".".to_string()]
}

fn default_ignored_paths() -> Vec<String> {
    vec![
        "target".to_string(),
        ".git".to_string(),
        "node_modules".to_string(),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct ProfileOverrides {
    pub require_scenario_command: Option<bool>,
    pub require_scenario_crate: Option<bool>,
    pub require_artifact_producer: Option<bool>,
    pub require_requirement_proof: Option<bool>,
    pub require_crate_scenario: Option<bool>,
    pub warnings_as_errors: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct ProfileRegistry {
    #[serde(default)]
    pub default: ProfileOverrides,
    #[serde(default)]
    pub ci: ProfileOverrides,
    #[serde(default)]
    pub strict: ProfileOverrides,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AtlasConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub profiles: ProfileRegistry,
    #[serde(default)]
    pub active_goal: Option<ActiveGoalConfig>,
}

fn default_schema_version() -> u32 {
    ATLAS_SCHEMA_VERSION
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            schema_version: ATLAS_SCHEMA_VERSION,
            discovery: DiscoveryConfig::default(),
            profiles: ProfileRegistry::default(),
            active_goal: None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ValidationProfile {
    Default,
    Ci,
    Strict,
}

impl ValidationProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Ci => "ci",
            Self::Strict => "strict",
        }
    }
}

impl fmt::Display for ValidationProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl FromStr for ValidationProfile {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "default" => Ok(Self::Default),
            "ci" => Ok(Self::Ci),
            "strict" => Ok(Self::Strict),
            other => Err(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSettings {
    pub require_scenario_command: bool,
    pub require_scenario_crate: bool,
    pub require_artifact_producer: bool,
    pub require_requirement_proof: bool,
    pub require_crate_scenario: bool,
    pub warnings_as_errors: bool,
}

impl ProfileSettings {
    pub fn builtin(profile: ValidationProfile) -> Self {
        match profile {
            ValidationProfile::Default => Self {
                require_scenario_command: true,
                require_scenario_crate: true,
                require_artifact_producer: false,
                require_requirement_proof: false,
                require_crate_scenario: false,
                warnings_as_errors: false,
            },
            ValidationProfile::Ci => Self {
                require_scenario_command: true,
                require_scenario_crate: true,
                require_artifact_producer: true,
                require_requirement_proof: true,
                require_crate_scenario: true,
                warnings_as_errors: false,
            },
            ValidationProfile::Strict => Self {
                require_scenario_command: true,
                require_scenario_crate: true,
                require_artifact_producer: true,
                require_requirement_proof: true,
                require_crate_scenario: true,
                warnings_as_errors: true,
            },
        }
    }

    pub fn apply_overrides(&mut self, overrides: &ProfileOverrides) {
        if let Some(value) = overrides.require_scenario_command {
            self.require_scenario_command = value;
        }
        if let Some(value) = overrides.require_scenario_crate {
            self.require_scenario_crate = value;
        }
        if let Some(value) = overrides.require_artifact_producer {
            self.require_artifact_producer = value;
        }
        if let Some(value) = overrides.require_requirement_proof {
            self.require_requirement_proof = value;
        }
        if let Some(value) = overrides.require_crate_scenario {
            self.require_crate_scenario = value;
        }
        if let Some(value) = overrides.warnings_as_errors {
            self.warnings_as_errors = value;
        }
    }
}

impl AtlasConfig {
    pub fn profile_settings(&self, profile: ValidationProfile) -> ProfileSettings {
        let mut settings = ProfileSettings::builtin(profile);
        settings.apply_overrides(&self.profiles.default);

        match profile {
            ValidationProfile::Default => {}
            ValidationProfile::Ci => settings.apply_overrides(&self.profiles.ci),
            ValidationProfile::Strict => settings.apply_overrides(&self.profiles.strict),
        }

        settings
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct QueryRequest {
    pub needle: String,
    pub kind: Option<NodeKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NodeMatch {
    pub score: u32,
    pub node: AtlasNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryResponse {
    pub needle: String,
    pub matches: Vec<NodeMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WhySubject {
    Id(AtlasId),
    Path(RepoRelativePath),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhyRequest {
    pub subject: WhySubject,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_recursive_touch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhyStep {
    pub node: AtlasNode,
    pub relationship: EdgeKind,
    pub direction: TraceDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhyResponse {
    pub root: AtlasNode,
    pub chain: Vec<WhyStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhyEnvelope {
    pub schema_version: u32,
    pub command: String,
    pub payload: WhyResponse,
}

impl WhyEnvelope {
    pub fn for_command(command: &str, payload: WhyResponse) -> Self {
        Self {
            schema_version: ATLAS_SCHEMA_VERSION,
            command: command.to_string(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChangedPath {
    pub path: RepoRelativePath,
    #[serde(default)]
    pub owners: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImpactRequest {
    pub paths: Vec<ChangedPath>,
    #[serde(default)]
    pub owners: std::collections::BTreeMap<RepoRelativePath, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImpactHit {
    pub node: AtlasNode,
    pub reason: String,
    #[serde(default)]
    pub owners: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ImpactMetrics {
    #[serde(default)]
    pub changed_path_count: usize,
    #[serde(default)]
    pub uncovered_path_count: usize,
    #[serde(default)]
    pub impacted_node_count: usize,
    #[serde(default)]
    pub owner_count: usize,
    #[serde(default)]
    pub missing_evidence_count: usize,
    #[serde(default)]
    pub scope_warning_count: usize,
    #[serde(default)]
    pub touched_only_path_count: usize,
    #[serde(default)]
    pub multi_owner_path_count: usize,
    #[serde(default)]
    pub coverage_percent: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImpactResponse {
    pub impacted: Vec<ImpactHit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_goal: Option<ActiveGoalConfig>,
    pub uncovered: Vec<ChangedPath>,
    #[serde(default)]
    pub changed_paths: Vec<ChangedPath>,
    #[serde(default)]
    pub metrics: ImpactMetrics,
    #[serde(default)]
    pub missing_evidence: Vec<AtlasDiagnostic>,
    #[serde(default)]
    pub scope_warnings: Vec<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImpactEnvelope {
    pub schema_version: u32,
    pub command: String,
    pub payload: ImpactResponse,
}

impl ImpactEnvelope {
    pub fn for_command(command: &str, payload: ImpactResponse) -> Self {
        Self {
            schema_version: ATLAS_SCHEMA_VERSION,
            command: command.to_string(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TraceDirection {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TraceRequest {
    pub start: AtlasId,
    pub direction: TraceDirection,
    pub max_depth: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TraceEdge {
    pub depth: usize,
    pub edge: AtlasEdge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TraceResponse {
    pub root: AtlasNode,
    pub nodes: Vec<AtlasNode>,
    pub edges: Vec<TraceEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub enum RenderFormat {
    Json,
    Markdown,
    GitHubSummary,
    ReviewPacket,
}

impl RenderFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::GitHubSummary => "gh-summary",
            Self::ReviewPacket => "review-packet",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Json => "atlas.json",
            Self::Markdown => "atlas.md",
            Self::GitHubSummary => "atlas.gh-summary.md",
            Self::ReviewPacket => "atlas.review-packet.md",
        }
    }
}

impl fmt::Display for RenderFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl FromStr for RenderFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            "gh-summary" => Ok(Self::GitHubSummary),
            "review-packet" => Ok(Self::ReviewPacket),
            other => Err(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_ids() {
        let id = AtlasId::parse("scen:build-emits-canonical-atlas").expect("valid id");
        assert_eq!(id.kind_prefix(), "scen");
        assert_eq!(id.slug(), "build-emits-canonical-atlas");
    }

    #[test]
    fn rejects_invalid_ids() {
        let err = AtlasId::parse("Scenario:Bad").expect_err("invalid id");
        assert!(matches!(err, AtlasIdError::InvalidKind { .. }));
    }

    #[test]
    fn repo_relative_path_normalizes_slashes() {
        let path = RepoRelativePath::new("crates\\foo\\src/lib.rs");
        assert_eq!(path.as_str(), "crates/foo/src/lib.rs");
    }

    #[test]
    fn path_selector_normalizes_slashes() {
        let selector = PathSelector::new("crates\\*\\src");
        assert_eq!(selector.pattern, "crates/*/src");
    }

    #[test]
    fn builds_profile_settings() {
        let config = AtlasConfig::default();
        let ci = config.profile_settings(ValidationProfile::Ci);
        assert!(ci.require_artifact_producer);
        assert!(!ci.warnings_as_errors);
    }

    #[test]
    fn why_subject_path_is_backward_compatible_from_legacy_json() {
        let request: WhyRequest =
            serde_json::from_str(r#"{"subject":{"Path":"crates/engine/src/lib.rs"}}"#)
                .expect("legacy why request should deserialize");
        assert_eq!(
            request.subject,
            WhySubject::Path("crates/engine/src/lib.rs".into())
        );
        assert!(!request.allow_recursive_touch);
    }
}
