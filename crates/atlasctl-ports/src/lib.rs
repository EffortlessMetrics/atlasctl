#![forbid(unsafe_code)]

use atlasctl_types::{AtlasGraph, ChangedPath, DiscoveredRepo, RenderFormat, WhyResponse};
use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DiscoverRequest {
    pub repo_root: Utf8PathBuf,
    pub config_path: Option<Utf8PathBuf>,
}

pub trait DiscoveryPort {
    fn discover(&self, request: &DiscoverRequest) -> Result<DiscoveredRepo, DiscoveryError>;
}

pub trait DiffPort {
    fn changed_paths(
        &self,
        repo_root: &camino::Utf8Path,
        base: &str,
        head: &str,
    ) -> Result<Vec<ChangedPath>, DiffError>;
}

pub trait OwnersPort {
    fn owners(
        &self,
        repo_root: &camino::Utf8Path,
        paths: &[atlasctl_types::RepoRelativePath],
    ) -> Result<
        std::collections::BTreeMap<atlasctl_types::RepoRelativePath, Vec<String>>,
        OwnersError,
    >;
}

pub trait RenderPort {
    fn render(&self, graph: &AtlasGraph, format: RenderFormat) -> Result<String, RenderError>;
    fn render_why(
        &self,
        response: &WhyResponse,
        format: RenderFormat,
    ) -> Result<String, RenderError>;
    fn render_impact(
        &self,
        response: &atlasctl_types::ImpactResponse,
        format: RenderFormat,
    ) -> Result<String, RenderError>;
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Error)]
pub enum DiffError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Error)]
pub enum OwnersError {
    #[error("{0}")]
    Message(String),
}
