#![forbid(unsafe_code)]

use atlasctl_codes::Severity;
use atlasctl_core::{compile_atlas, query_graph, trace_graph};
use atlasctl_ports::{DiscoverRequest, DiscoveryError, DiscoveryPort, RenderError, RenderPort};
use atlasctl_types::{
    AtlasGraph, QueryRequest, QueryResponse, RenderFormat, TraceRequest, TraceResponse,
    ValidationProfile,
};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub repo_root: Utf8PathBuf,
    pub config_path: Option<Utf8PathBuf>,
    pub profile: ValidationProfile,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub compile: CompileOptions,
    pub formats: Vec<RenderFormat>,
}

#[derive(Debug, Clone)]
pub struct BuildOutcome {
    pub graph: AtlasGraph,
    pub rendered: BTreeMap<RenderFormat, String>,
    pub has_errors: bool,
}

#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub graph: AtlasGraph,
    pub has_errors: bool,
}

#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub compile: CompileOptions,
    pub request: QueryRequest,
}

#[derive(Debug, Clone)]
pub struct QueryOutcome {
    pub graph: AtlasGraph,
    pub response: QueryResponse,
}

#[derive(Debug, Clone)]
pub struct TraceOptions {
    pub compile: CompileOptions,
    pub request: TraceRequest,
}

#[derive(Debug, Clone)]
pub struct TraceOutcome {
    pub graph: AtlasGraph,
    pub response: Option<TraceResponse>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    Render(#[from] RenderError),
}

pub struct AtlasService<D, R> {
    discovery: D,
    renderer: R,
}

impl<D, R> AtlasService<D, R> {
    pub fn new(discovery: D, renderer: R) -> Self {
        Self {
            discovery,
            renderer,
        }
    }
}

impl<D, R> AtlasService<D, R>
where
    D: DiscoveryPort,
    R: RenderPort,
{
    pub fn build(&self, options: &BuildOptions) -> Result<BuildOutcome, AppError> {
        let graph = self.compile(&options.compile)?;
        let mut rendered = BTreeMap::new();

        for format in &options.formats {
            let text = self.renderer.render(&graph, *format)?;
            rendered.insert(*format, text);
        }

        let has_errors = graph
            .diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error);

        Ok(BuildOutcome {
            graph,
            rendered,
            has_errors,
        })
    }

    pub fn check(&self, options: &CompileOptions) -> Result<CheckOutcome, AppError> {
        let graph = self.compile(options)?;
        let has_errors = graph
            .diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error);

        Ok(CheckOutcome { graph, has_errors })
    }

    pub fn query(&self, options: &QueryOptions) -> Result<QueryOutcome, AppError> {
        let graph = self.compile(&options.compile)?;
        let response = query_graph(&graph, &options.request);

        Ok(QueryOutcome { graph, response })
    }

    pub fn trace(&self, options: &TraceOptions) -> Result<TraceOutcome, AppError> {
        let graph = self.compile(&options.compile)?;
        let response = trace_graph(&graph, &options.request);

        Ok(TraceOutcome { graph, response })
    }

    fn compile(&self, options: &CompileOptions) -> Result<AtlasGraph, AppError> {
        let discovered = self.discovery.discover(&DiscoverRequest {
            repo_root: options.repo_root.clone(),
            config_path: options.config_path.clone(),
        })?;

        Ok(compile_atlas(discovered, options.profile))
    }
}
