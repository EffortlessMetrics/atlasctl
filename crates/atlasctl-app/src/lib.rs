#![forbid(unsafe_code)]

use atlasctl_core::{
    compile_atlas, impacted_graph, owning_nodes_for_path, query_graph, trace_graph, why_graph,
};
use atlasctl_types::{
    AtlasGraph, ChangedPath, DiscoveredRepo, ImpactRequest, ImpactResponse, QueryRequest,
    QueryResponse, RenderFormat, RepoRelativePath, Severity, TraceRequest, TraceResponse,
    ValidationProfile, WhyRequest, WhyResponse,
};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;
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
        paths: &[RepoRelativePath],
    ) -> Result<std::collections::BTreeMap<RepoRelativePath, Vec<String>>, OwnersError>;
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
        response: &ImpactResponse,
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

pub struct AtlasService<D: DiscoveryPort, R: RenderPort, G: DiffPort, O: OwnersPort> {
    pub discovery: D,
    pub renderer: R,
    pub diff: G,
    pub owners: O,
}

impl<D: DiscoveryPort, R: RenderPort, G: DiffPort, O: OwnersPort> AtlasService<D, R, G, O> {
    pub fn new(discovery: D, renderer: R, diff: G, owners: O) -> Self {
        Self {
            discovery,
            renderer,
            diff,
            owners,
        }
    }

    pub fn build(&self, options: &BuildOptions) -> Result<BuildOutcome, AppError> {
        let graph = self.compile(&options.compile)?;
        let has_errors = graph
            .diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error);

        let mut rendered = BTreeMap::new();
        for format in &options.formats {
            let content = self.renderer.render(&graph, *format)?;
            rendered.insert(*format, content);
        }

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

    pub fn doctor(&self, options: &CompileOptions) -> Result<CheckOutcome, AppError> {
        // For now, doctor is a semantic alias for check,
        // as all doctor rules are integrated into discovery and compilation.
        self.check(options)
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

    pub fn why(&self, options: &WhyOptions) -> Result<WhyOutcome, AppError> {
        let graph = self.compile(&options.compile)?;
        let response = why_graph(&graph, &options.request);

        Ok(WhyOutcome { graph, response })
    }

    pub fn impacted(&self, options: &ImpactOptions) -> Result<ImpactOutcome, AppError> {
        let graph = self.compile(&options.compile)?;
        let mut paths = match &options.request {
            ImpactSource::Paths(paths) => paths.clone(),
            ImpactSource::Diff { base, head } => {
                self.diff
                    .changed_paths(&options.compile.repo_root, base, head)?
            }
        };

        let repo_paths: Vec<_> = paths.iter().map(|p| p.path.clone()).collect();
        let owners = self
            .owners
            .owners(&options.compile.repo_root, &repo_paths)?;

        for changed in &mut paths {
            if let Some(found) = owners.get(&changed.path) {
                changed.owners = found.clone();
                continue;
            }

            let fallback_owners: Vec<_> = owning_nodes_for_path(&graph, &changed.path)
                .into_iter()
                .map(|id| id.to_string())
                .collect();
            if !fallback_owners.is_empty() {
                changed.owners = fallback_owners;
            }
        }

        let response = impacted_graph(&graph, &ImpactRequest { paths, owners });
        let (has_uncovered_warning, has_uncovered_error) =
            coverage_severity(options.compile.profile, !response.uncovered.is_empty());

        Ok(ImpactOutcome {
            graph,
            response,
            has_uncovered_warning,
            has_uncovered_error,
        })
    }

    fn compile(&self, options: &CompileOptions) -> Result<AtlasGraph, AppError> {
        let discovered = self.discovery.discover(&DiscoverRequest {
            repo_root: options.repo_root.clone(),
            config_path: options.config_path.clone(),
        })?;

        Ok(compile_atlas(discovered, options.profile))
    }
}

pub struct CompileOptions {
    pub repo_root: Utf8PathBuf,
    pub config_path: Option<Utf8PathBuf>,
    pub profile: ValidationProfile,
}

pub struct BuildOptions {
    pub compile: CompileOptions,
    pub formats: Vec<RenderFormat>,
}

pub struct BuildOutcome {
    pub graph: AtlasGraph,
    pub rendered: BTreeMap<RenderFormat, String>,
    pub has_errors: bool,
}

pub struct CheckOutcome {
    pub graph: AtlasGraph,
    pub has_errors: bool,
}

pub struct QueryOptions {
    pub compile: CompileOptions,
    pub request: QueryRequest,
}

pub struct QueryOutcome {
    pub graph: AtlasGraph,
    pub response: QueryResponse,
}

pub struct TraceOptions {
    pub compile: CompileOptions,
    pub request: TraceRequest,
}

pub struct TraceOutcome {
    pub graph: AtlasGraph,
    pub response: Option<TraceResponse>,
}

pub struct WhyOptions {
    pub compile: CompileOptions,
    pub request: WhyRequest,
}

pub struct WhyOutcome {
    pub graph: AtlasGraph,
    pub response: Option<WhyResponse>,
}

pub enum ImpactSource {
    Paths(Vec<ChangedPath>),
    Diff { base: String, head: String },
}

pub struct ImpactOptions {
    pub compile: CompileOptions,
    pub request: ImpactSource,
}

pub struct ImpactOutcome {
    pub graph: AtlasGraph,
    pub response: ImpactResponse,
    pub has_uncovered_warning: bool,
    pub has_uncovered_error: bool,
}

fn coverage_severity(profile: ValidationProfile, has_uncovered: bool) -> (bool, bool) {
    if !has_uncovered {
        return (false, false);
    }

    match profile {
        ValidationProfile::Strict => (false, true),
        ValidationProfile::Default | ValidationProfile::Ci => (true, false),
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("discovery failed: {0}")]
    Discovery(#[from] DiscoveryError),
    #[error("rendering failed: {0}")]
    Render(#[from] RenderError),
    #[error("diff failed: {0}")]
    Diff(#[from] DiffError),
    #[error("owners failed: {0}")]
    Owners(#[from] OwnersError),
}
