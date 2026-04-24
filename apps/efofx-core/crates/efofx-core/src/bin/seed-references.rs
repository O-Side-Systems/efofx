//! `seed-references` — one-shot operational binary for loading platform
//! reference classes and reference projects from JSON fixtures.
//!
//! Reads bundle files from `config/reference-data/*.json` (override with
//! `--dir <path>`), validates them, and upserts platform-scoped rows
//! (`tenant_id: null`) into the `reference_classes` and `reference_projects`
//! collections.
//!
//! Modes:
//! - *default:* idempotent upsert by `(tenant_id: null, name)` for classes
//!   and `(tenant_id: null, project_id)` for projects
//! - `--dry-run:` validate + summarize, no writes
//! - `--clean:` delete every platform row first, then upsert
//!
//! Tenant-scoped rows are **never** touched.
//!
//! Connection config is loaded through `AppConfig` so Mongo URI / db_name /
//! Supabase settings match the server. Secrets come from environment
//! variables (`EFOFX_MONGO__URI`). Supabase config keys must still be
//! present but aren't contacted.
//!
//! The fixture schema is intentionally terser than the on-wire Mongo shape:
//! - No `tenant_id` (always platform-scoped → `null`)
//! - No `created_at` (filled with `now` at seed time)
//! - Projects omit `reference_class` and `is_active` (auto-filled from the
//!   bundle's class name / `true`)

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use efofx_config::AppConfig;
use efofx_domain::{CostDistribution, ReferenceClass, ReferenceProject, TimelineDistribution};
use efofx_storage::{MongoAdapter, ReferenceRepo, UpsertStats};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{info, warn};

/// Parsed CLI options. Hand-rolled to avoid pulling `clap` for three flags.
struct Args {
    dry_run: bool,
    clean: bool,
    dir: PathBuf,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut dry_run = false;
        let mut clean = false;
        let mut dir = PathBuf::from("config/reference-data");
        let mut iter = std::env::args().skip(1);
        while let Some(a) = iter.next() {
            match a.as_str() {
                "--dry-run" => dry_run = true,
                "--clean" => clean = true,
                "--dir" => {
                    dir = iter
                        .next()
                        .map(PathBuf::from)
                        .context("--dir requires a path argument")?;
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                other => bail!("unknown argument: {other} (try --help)"),
            }
        }
        Ok(Self {
            dry_run,
            clean,
            dir,
        })
    }
}

fn print_help() {
    println!(
        "\
seed-references — upsert platform reference data from JSON fixtures

USAGE:
    seed-references [--dir <path>] [--dry-run] [--clean]

FLAGS:
    --dir <path>  Directory of bundle files (default: config/reference-data)
    --dry-run     Validate + summarize without writing
    --clean       Delete all platform rows before upserting
    -h, --help    Print this help

Connection config is loaded via AppConfig (same file + env as efofx-core).
Requires EFOFX_MONGO__URI, EFOFX_SUPABASE__URL, EFOFX_SUPABASE__ISSUER."
    );
}

/// Fixture bundle: one reference class plus its seed projects.
#[derive(Debug, Deserialize)]
struct FixtureBundle {
    class: FixtureClass,
    #[serde(default)]
    projects: Vec<FixtureProject>,
}

#[derive(Debug, Deserialize)]
struct FixtureClass {
    category: String,
    subcategory: String,
    name: String,
    description: String,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    regions: Vec<String>,
    #[serde(default)]
    attributes: BTreeMap<String, serde_json::Value>,
    cost_distribution: CostDistribution,
    timeline_distribution: TimelineDistribution,
    cost_breakdown_template: BTreeMap<String, f64>,
    #[serde(default)]
    is_synthetic: bool,
    validation_source: String,
}

#[derive(Debug, Deserialize)]
struct FixtureProject {
    project_id: String,
    region: String,
    description: String,
    #[serde(default)]
    size_sqft: Option<f64>,
    total_cost: f64,
    timeline_weeks: u32,
    team_size: u32,
    cost_breakdown: BTreeMap<String, f64>,
    #[serde(with = "time::serde::rfc3339")]
    completion_date: OffsetDateTime,
    quality_score: f64,
    source: String,
    #[serde(default)]
    metadata: BTreeMap<String, serde_json::Value>,
}

impl FixtureClass {
    fn into_domain(self, now: OffsetDateTime) -> ReferenceClass {
        ReferenceClass {
            id: None,
            tenant_id: None,
            category: self.category,
            subcategory: self.subcategory,
            name: self.name,
            description: self.description,
            keywords: self.keywords,
            regions: self.regions,
            attributes: self.attributes,
            cost_distribution: self.cost_distribution,
            timeline_distribution: self.timeline_distribution,
            cost_breakdown_template: self.cost_breakdown_template,
            is_synthetic: self.is_synthetic,
            validation_source: self.validation_source,
            created_at: now,
            updated_at: None,
        }
    }
}

impl FixtureProject {
    fn into_domain(self, class_name: &str, now: OffsetDateTime) -> ReferenceProject {
        ReferenceProject {
            id: None,
            tenant_id: None,
            project_id: self.project_id,
            reference_class: class_name.to_string(),
            region: self.region,
            description: self.description,
            size_sqft: self.size_sqft,
            total_cost: self.total_cost,
            timeline_weeks: self.timeline_weeks,
            team_size: self.team_size,
            cost_breakdown: self.cost_breakdown,
            completion_date: self.completion_date,
            quality_score: self.quality_score,
            source: self.source,
            metadata: self.metadata,
            is_active: true,
            created_at: now,
            updated_at: None,
        }
    }
}

fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse()?;
    // Spin up a Tokio runtime explicitly so `main` itself stays sync and the
    // error path in `Args::parse` can short-circuit before we touch async.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    rt.block_on(run(args))
}

async fn run(args: Args) -> Result<()> {
    let cfg = AppConfig::load().context("load AppConfig (check EFOFX_* env vars)")?;
    info!(
        dir = %args.dir.display(),
        db = %cfg.mongo.db_name,
        dry_run = args.dry_run,
        clean = args.clean,
        "seed-references starting"
    );

    let bundles = load_bundles(&args.dir)?;
    validate(&bundles)?;
    let now = OffsetDateTime::now_utc();
    let prepared: Vec<(ReferenceClass, Vec<ReferenceProject>)> = bundles
        .into_iter()
        .map(|(path, b)| {
            let class_name = b.class.name.clone();
            let class = b.class.into_domain(now);
            let projects = b
                .projects
                .into_iter()
                .map(|p| p.into_domain(&class_name, now))
                .collect();
            info!(fixture = %path.display(), class = %class.name, "bundle prepared");
            (class, projects)
        })
        .collect();

    if args.dry_run {
        let totals = summarize(&prepared);
        info!(
            classes = totals.classes,
            projects = totals.projects,
            "dry-run complete — no writes performed"
        );
        return Ok(());
    }

    let mongo = MongoAdapter::connect(&cfg.mongo)
        .await
        .context("connect to mongo")?;
    let repo = ReferenceRepo::new(mongo);
    repo.ensure_indexes()
        .await
        .context("ensure reference_* indexes")?;

    if args.clean {
        let dc = repo
            .delete_platform_classes()
            .await
            .context("delete platform classes")?;
        let dp = repo
            .delete_platform_projects()
            .await
            .context("delete platform projects")?;
        warn!(
            classes_deleted = dc,
            projects_deleted = dp,
            "clean complete"
        );
    }

    let mut class_stats = UpsertStats::default();
    let mut project_stats = UpsertStats::default();

    for (class, projects) in &prepared {
        let r = repo
            .upsert_platform_class(class)
            .await
            .with_context(|| format!("upsert class {}", class.name))?;
        class_stats.observe(&r);
        info!(
            class = %class.name,
            upserted = r.upserted_id.is_some(),
            modified = r.modified_count,
            "class upserted"
        );
        for p in projects {
            let r = repo
                .upsert_platform_project(p)
                .await
                .with_context(|| format!("upsert project {}", p.project_id))?;
            project_stats.observe(&r);
        }
    }

    info!(
        classes_inserted = class_stats.inserted,
        classes_modified = class_stats.modified,
        classes_unchanged = class_stats.unchanged,
        projects_inserted = project_stats.inserted,
        projects_modified = project_stats.modified,
        projects_unchanged = project_stats.unchanged,
        "seed-references done"
    );
    Ok(())
}

/// Totals for `--dry-run` reporting.
struct Totals {
    classes: usize,
    projects: usize,
}

fn summarize(prepared: &[(ReferenceClass, Vec<ReferenceProject>)]) -> Totals {
    Totals {
        classes: prepared.len(),
        projects: prepared.iter().map(|(_, ps)| ps.len()).sum(),
    }
}

fn load_bundles(dir: &Path) -> Result<Vec<(PathBuf, FixtureBundle)>> {
    if !dir.exists() {
        bail!("fixture directory does not exist: {}", dir.display());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    // Deterministic order so logs stay stable across runs / filesystems.
    paths.sort();

    if paths.is_empty() {
        bail!("no *.json fixtures found under {}", dir.display());
    }

    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let bundle: FixtureBundle = serde_json::from_str(&raw)
            .with_context(|| format!("parse fixture {}", path.display()))?;
        out.push((path, bundle));
    }
    Ok(out)
}

/// Cross-bundle + per-bundle validation:
/// - class names unique across all bundles
/// - project IDs unique across all bundles
/// - cost_breakdown_template sums to 1.0 (class-level)
/// - each project's cost_breakdown keys are a subset of the class template
fn validate(bundles: &[(PathBuf, FixtureBundle)]) -> Result<()> {
    let mut class_names: BTreeSet<&str> = BTreeSet::new();
    let mut project_ids: BTreeSet<&str> = BTreeSet::new();

    for (path, b) in bundles {
        if !class_names.insert(b.class.name.as_str()) {
            bail!(
                "duplicate class name {:?} (last seen in {})",
                b.class.name,
                path.display()
            );
        }
        let total: f64 = b.class.cost_breakdown_template.values().sum();
        if (total - 1.0).abs() > 0.01 {
            bail!(
                "{}: class {:?} cost_breakdown_template sums to {:.4}, expected ~1.0",
                path.display(),
                b.class.name,
                total
            );
        }
        let template_keys: BTreeSet<&str> = b
            .class
            .cost_breakdown_template
            .keys()
            .map(String::as_str)
            .collect();
        for p in &b.projects {
            if !project_ids.insert(p.project_id.as_str()) {
                bail!(
                    "duplicate project_id {:?} (from {})",
                    p.project_id,
                    path.display()
                );
            }
            for k in p.cost_breakdown.keys() {
                if !template_keys.contains(k.as_str()) {
                    bail!(
                        "{}: project {:?} cost_breakdown key {:?} is not in class template",
                        path.display(),
                        p.project_id,
                        k
                    );
                }
            }
            if !(0.0..=1.0).contains(&p.quality_score) {
                bail!(
                    "{}: project {:?} quality_score {} out of [0,1]",
                    path.display(),
                    p.project_id,
                    p.quality_score
                );
            }
        }
    }
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,seed_references=debug,efofx_storage=info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false))
        .init();
}
