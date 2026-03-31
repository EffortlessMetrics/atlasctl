#![forbid(unsafe_code)]

use atlasctl_types::{AtlasGraph, DiscoveredRepo, RenderFormat};
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

pub trait RenderPort {
    fn render(&self, graph: &AtlasGraph, format: RenderFormat) -> Result<String, RenderError>;
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
