use anyhow::{Result, Context, bail};
use clap::{Args, Subcommand};
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use tokio::task::JoinSet;
use toml;
#[derive(Debug, Args)]
pub struct DdrArgs {
    #[command(subcommand)]
    pub action: Option<DdrAction>,
}
#[derive(Debug, Subcommand)]
pub enum DdrAction {
    Build {
        #[arg(short, long)]
        image: Option<String>,
        #[arg(short = 't', long)]
        target: Vec<String>,
        #[arg(short = 'j', long, default_value = "16")]
        jobs: usize,
        #[arg(short = 'c', long, default_value = "ddr.toml")]
        config: PathBuf,
        #[arg(long)]
        use_config: bool,
    },
    Generate {
        #[arg(short, long, default_value = "ddr.toml")]
        output: PathBuf,
        #[arg(long)]
        auto: bool,
    },
    Status { #[arg(short, long)] verbose: bool },
    Clean { #[arg(long)] all: bool, #[arg(short, long)] project: Option<String> },
    Validate { #[arg(short, long, default_value = "ddr.toml")] config: PathBuf },
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DdrConfig {
    pub project: ProjectConfig,
    pub docker: DockerConfig,
    pub targets: HashMap<String, TargetConfig>,
    pub parallel: ParallelConfig,
    pub cache: Option<CacheConfig>,
    pub artifacts: Option<ArtifactConfig>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub workspace: Option<PathBuf>,
    pub cargo_toml: PathBuf,
    pub src_dir: PathBuf,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DockerConfig {
    pub registry: Option<String>,
    pub build_args: Option<HashMap<String, String>>,
    pub network: Option<String>,
    pub volumes: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetConfig {
    pub triple: String,
    pub image: String,
    pub dockerfile: Option<PathBuf>,
    pub features: Option<Vec<String>>,
    pub rustflags: Option<String>,
    pub linker: Option<String>,
    pub strip: Option<bool>,
    pub upx: Option<bool>,
    pub test: Option<bool>,
    pub bench: Option<bool>,
    pub priority: Option<u8>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelConfig {
    pub max_jobs: usize,
    pub batch_size: Option<usize>,
    pub timeout_minutes: Option<u64>,
    pub retry_failed: Option<u8>,
    pub fail_fast: Option<bool>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub registry_cache: Option<bool>,
    pub cargo_cache: Option<PathBuf>,
    pub sccache: Option<bool>,
    pub cache_from: Option<Vec<String>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactConfig {
    pub output_dir: PathBuf,
    pub compress: Option<bool>,
    pub checksum: Option<bool>,
    pub manifest: Option<bool>,
}
#[derive(Debug, Clone)]
pub struct BuildJob {
    pub id: String,
    pub target: String,
    pub image: String,
    pub status: BuildStatus,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub container_id: Option<String>,
    pub output: Option<String>,
    pub artifact_path: Option<PathBuf>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum BuildStatus {
    Queued,
    Running,
    Success,
    Failed(String),
    Skipped,
    Retrying(u8),
}
pub struct BuildOrchestrator {
    config: DdrConfig,
    jobs: Arc<RwLock<HashMap<String, BuildJob>>>,
    semaphore: Arc<Semaphore>,
    progress: MultiProgress,
}
impl BuildOrchestrator {
    pub fn new(config: DdrConfig) -> Self {
        let max_jobs = config.parallel.max_jobs.min(32).max(1);
        Self {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_jobs)),
            progress: MultiProgress::new(),
        }
    }
    pub async fn run(&self) -> Result<BuildReport> {
        let start_time = Instant::now();
        let mut job_handles = JoinSet::new();
        let main_pb = self.create_main_progress_bar(self.config.targets.len());
        let mut sorted_targets: Vec<_> = self.config.targets.iter().collect();
        sorted_targets.sort_by_key(|(_, tc)| tc.priority.unwrap_or(100));
        for (target_name, target_config) in sorted_targets {
            let job = BuildJob {
                id: format!("{}_{}", self.config.project.name, target_name),
                target: target_name.clone(),
                image: target_config.image.clone(),
                status: BuildStatus::Queued,
                start_time: None,
                end_time: None,
                container_id: None,
                output: None,
                artifact_path: None,
            };
            self.jobs.write().await.insert(job.id.clone(), job.clone());
            let orchestrator = self.clone_for_job();
            let target_config = target_config.clone();
            let pb = self.create_target_progress_bar(&target_name);
            job_handles
                .spawn(async move {
                    orchestrator.run_build_job(job, target_config, pb).await
                });
        }
        let mut results = Vec::new();
        while let Some(result) = job_handles.join_next().await {
            match result {
                Ok(Ok(job)) => results.push(job),
                Ok(Err(e)) => eprintln!("Build job failed: {}", e),
                Err(e) => eprintln!("Task panicked: {}", e),
            }
            main_pb.inc(1);
        }
        main_pb.finish_with_message("All builds complete!");
        Ok(self.generate_report(results, start_time.elapsed()).await)
    }
    async fn run_build_job(
        &self,
        mut job: BuildJob,
        target_config: TargetConfig,
        pb: ProgressBar,
    ) -> Result<BuildJob> {
        let _permit = self.semaphore.acquire().await?;
        pb.set_message(format!("Building {}", job.target));
        job.status = BuildStatus::Running;
        job.start_time = Some(Instant::now());
        self.update_job(&job).await;
        let dockerfile = if let Some(ref df) = target_config.dockerfile {
            fs::read_to_string(df)?
        } else {
            self.generate_dockerfile(&target_config)?
        };
        let image_tag = format!("{}:{}", job.id, chrono::Utc::now().timestamp());
        self.build_docker_image(&image_tag, &dockerfile, &pb).await?;
        let container_id = self.run_container(&image_tag, &target_config, &pb).await?;
        job.container_id = Some(container_id.clone());
        let artifact_path = self.extract_artifacts(&container_id, &job.target).await?;
        job.artifact_path = Some(artifact_path);
        self.cleanup_container(&container_id).await?;
        job.status = BuildStatus::Success;
        job.end_time = Some(Instant::now());
        self.update_job(&job).await;
        pb.finish_with_message(format!("✅ {} complete", job.target));
        Ok(job)
    }
    fn generate_dockerfile(&self, target: &TargetConfig) -> Result<String> {
        let mut dockerfile = String::new();
        dockerfile.push_str(&format!("FROM {} AS builder\n\n", target.image));
        dockerfile.push_str("RUN apt-get update && apt-get install -y \\\n");
        dockerfile.push_str("    build-essential \\\n");
        dockerfile.push_str("    pkg-config \\\n");
        dockerfile.push_str("    libssl-dev \\\n");
        dockerfile.push_str("    && rm -rf /var/lib/apt/lists/*\n\n");
        if !target.image.contains("rust") {
            dockerfile
                .push_str(
                    "RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y\n",
                );
            dockerfile.push_str("ENV PATH=/root/.cargo/bin:$PATH\n\n");
        }
        dockerfile.push_str(&format!("RUN rustup target add {}\n\n", target.triple));
        if let Some(cache) = &self.config.cache {
            if cache.sccache.unwrap_or(false) {
                dockerfile.push_str("RUN cargo install sccache\n");
                dockerfile.push_str("ENV RUSTC_WRAPPER=sccache\n\n");
            }
        }
        dockerfile.push_str("WORKDIR /build\n\n");
        dockerfile.push_str("COPY Cargo.toml Cargo.lock* ./\n");
        dockerfile.push_str("COPY src ./src\n\n");
        let mut build_cmd = format!("cargo build --release --target {}", target.triple);
        if let Some(features) = &target.features {
            build_cmd.push_str(&format!(" --features {}", features.join(",")));
        }
        dockerfile.push_str(&format!("RUN {}\n\n", build_cmd));
        if target.strip.unwrap_or(true) {
            dockerfile
                .push_str(&format!("RUN strip target/{}/release/*\n\n", target.triple));
        }
        if target.upx.unwrap_or(false) {
            dockerfile.push_str("RUN apt-get update && apt-get install -y upx\n");
            dockerfile
                .push_str(
                    &format!(
                        "RUN upx --best --lzma target/{}/release/* || true\n\n", target
                        .triple
                    ),
                );
        }
        dockerfile.push_str("FROM scratch AS final\n");
        dockerfile
            .push_str(
                &format!(
                    "COPY --from=builder /build/target/{}/release/* /artifacts/\n",
                    target.triple
                ),
            );
        Ok(dockerfile)
    }
    async fn build_docker_image(
        &self,
        tag: &str,
        dockerfile: &str,
        pb: &ProgressBar,
    ) -> Result<()> {
        pb.set_message("Building Docker image...");
        let temp_dockerfile = format!(
            ".ddr_dockerfile_{}", chrono::Utc::now().timestamp()
        );
        fs::write(&temp_dockerfile, dockerfile)?;
        let output = Command::new("docker")
            .args(&["build", "-t", tag, "-f", &temp_dockerfile, "."])
            .output()?;
        let _ = fs::remove_file(&temp_dockerfile);
        if !output.status.success() {
            bail!("Docker build failed: {}", String::from_utf8_lossy(& output.stderr));
        }
        Ok(())
    }
    async fn run_container(
        &self,
        image: &str,
        target: &TargetConfig,
        pb: &ProgressBar,
    ) -> Result<String> {
        pb.set_message("Running build container...");
        let mut cmd = Command::new("docker");
        cmd.args(&["run", "-d"]);
        if let Some(volumes) = &self.config.docker.volumes {
            for vol in volumes {
                cmd.args(&["-v", vol]);
            }
        }
        if let Some(env) = &self.config.docker.env {
            for (key, val) in env {
                cmd.args(&["-e", &format!("{}={}", key, val)]);
            }
        }
        if let Some(rustflags) = &target.rustflags {
            cmd.args(&["-e", &format!("RUSTFLAGS={}", rustflags)]);
        }
        cmd.arg(image);
        cmd.arg("sleep");
        cmd.arg("3600");
        let output = cmd.output()?;
        if !output.status.success() {
            bail!(
                "Failed to start container: {}", String::from_utf8_lossy(& output.stderr)
            );
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }
    async fn extract_artifacts(
        &self,
        container_id: &str,
        target: &str,
    ) -> Result<PathBuf> {
        let artifacts_dir = self
            .config
            .artifacts
            .as_ref()
            .map(|a| a.output_dir.clone())
            .unwrap_or_else(|| PathBuf::from("target/ddr"));
        fs::create_dir_all(&artifacts_dir)?;
        let target_dir = artifacts_dir.join(target);
        fs::create_dir_all(&target_dir)?;
        let output = Command::new("docker")
            .args(
                &[
                    "cp",
                    &format!("{}:/artifacts/.", container_id),
                    &target_dir.to_string_lossy(),
                ],
            )
            .output()?;
        if !output.status.success() {
            bail!(
                "Failed to extract artifacts: {}", String::from_utf8_lossy(& output
                .stderr)
            );
        }
        if let Some(artifacts) = &self.config.artifacts {
            if artifacts.checksum.unwrap_or(false) {
                self.generate_checksums(&target_dir)?;
            }
        }
        Ok(target_dir)
    }
    async fn cleanup_container(&self, container_id: &str) -> Result<()> {
        Command::new("docker").args(&["rm", "-f", container_id]).output()?;
        Ok(())
    }
    async fn update_job(&self, job: &BuildJob) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job.clone());
    }
    fn create_main_progress_bar(&self, total: usize) -> ProgressBar {
        let pb = self.progress.add(ProgressBar::new(total as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    }
    fn create_target_progress_bar(&self, target: &str) -> ProgressBar {
        let pb = self.progress.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap(),
        );
        pb.set_message(format!("⏳ {}", target));
        pb
    }
    fn clone_for_job(&self) -> Self {
        Self {
            config: self.config.clone(),
            jobs: Arc::clone(&self.jobs),
            semaphore: Arc::clone(&self.semaphore),
            progress: MultiProgress::new(),
        }
    }
    async fn generate_report(
        &self,
        jobs: Vec<BuildJob>,
        total_time: Duration,
    ) -> BuildReport {
        let jobs_guard = self.jobs.read().await;
        let successful = jobs
            .iter()
            .filter(|j| j.status == BuildStatus::Success)
            .count();
        let failed = jobs
            .iter()
            .filter(|j| matches!(j.status, BuildStatus::Failed(_)))
            .count();
        BuildReport {
            total_jobs: jobs.len(),
            successful,
            failed,
            skipped: 0,
            total_time,
            jobs: jobs.clone(),
            artifacts: jobs.iter().filter_map(|j| j.artifact_path.clone()).collect(),
        }
    }
    fn generate_checksums(&self, dir: &Path) -> Result<()> {
        use sha2::{Sha256, Digest};
        use std::io::Read;
        let mut checksums = String::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let path = entry.path();
                let mut file = fs::File::open(&path)?;
                let mut hasher = Sha256::new();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                hasher.update(&buffer);
                let result = hasher.finalize();
                checksums
                    .push_str(
                        &format!(
                            "{:x}  {}\n", result, path.file_name().unwrap()
                            .to_string_lossy()
                        ),
                    );
            }
        }
        fs::write(dir.join("SHA256SUMS"), checksums)?;
        Ok(())
    }
}
#[derive(Debug)]
pub struct BuildReport {
    pub total_jobs: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_time: Duration,
    pub jobs: Vec<BuildJob>,
    pub artifacts: Vec<PathBuf>,
}
impl BuildReport {
    pub fn print(&self) {
        println!(
            "\n{}",
            "═══════════════════════════════════════"
            .bright_blue()
        );
        println!("{}", "       DDR BUILD REPORT".bright_white().bold());
        println!(
            "{}",
            "═══════════════════════════════════════"
            .bright_blue()
        );
        println!("\n📊 {} Summary", "Build".bright_cyan());
        println!("   Total Jobs:  {}", self.total_jobs.to_string().bright_white());
        println!("   ✅ Success:  {}", self.successful.to_string().bright_green());
        println!("   ❌ Failed:   {}", self.failed.to_string().bright_red());
        println!("   ⏩ Skipped:  {}", self.skipped.to_string().bright_yellow());
        println!("   ⏱️  Duration:  {:.2}s", self.total_time.as_secs_f64());
        if !self.jobs.is_empty() {
            println!("\n📦 {} Details:", "Target".bright_cyan());
            for job in &self.jobs {
                let status_icon = match job.status {
                    BuildStatus::Success => "✅",
                    BuildStatus::Failed(_) => "❌",
                    BuildStatus::Skipped => "⏩",
                    _ => "❓",
                };
                let duration = if let (Some(start), Some(end)) = (
                    job.start_time,
                    job.end_time,
                ) {
                    format!("{:.2}s", (end - start).as_secs_f64())
                } else {
                    "N/A".to_string()
                };
                println!(
                    "   {} {} ({})", status_icon, job.target.bright_white(), duration
                    .bright_black()
                );
            }
        }
        if !self.artifacts.is_empty() {
            println!("\n🎯 {} Generated:", "Artifacts".bright_cyan());
            for artifact in &self.artifacts {
                println!("   📁 {}", artifact.display());
            }
        }
        println!(
            "\n{}",
            "═══════════════════════════════════════"
            .bright_blue()
        );
    }
}
pub async fn handle_ddr(action: Option<DdrAction>) -> Result<()> {
    match action {
        Some(DdrAction::Build { image, target, jobs, config, use_config }) => {
            handle_build(image, target, jobs, config, use_config).await
        }
        Some(DdrAction::Generate { output, auto }) => handle_generate(output, auto).await,
        Some(DdrAction::Status { verbose }) => handle_status(verbose).await,
        Some(DdrAction::Clean { all, project }) => handle_clean(all, project).await,
        Some(DdrAction::Validate { config }) => handle_validate(config).await,
        None => {
            if Path::new("ddr.toml").exists() {
                handle_build(None, vec![], 16, PathBuf::from("ddr.toml"), true).await
            } else {
                println!("No ddr.toml found. Run 'cm ddr generate' to create one.");
                Ok(())
            }
        }
    }
}
async fn handle_build(
    image: Option<String>,
    targets: Vec<String>,
    jobs: usize,
    config_path: PathBuf,
    use_config: bool,
) -> Result<()> {
    println!("{}", "🚀 Starting DDR Build Orchestration".bright_cyan().bold());
    let config = if config_path.exists() && use_config {
        let content = fs::read_to_string(&config_path)?;
        toml::from_str::<DdrConfig>(&content)?
    } else if config_path.exists() {
        println!("Found existing config: {}", config_path.display());
        print!("Use existing config? [Y/n]: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "n" {
            let content = fs::read_to_string(&config_path)?;
            toml::from_str::<DdrConfig>(&content)?
        } else {
            generate_config(image, targets, jobs)?
        }
    } else {
        println!("No config found. Generating default configuration...");
        let config = generate_config(image, targets, jobs)?;
        let toml_str = toml::to_string_pretty(&config)?;
        fs::write(&config_path, toml_str)?;
        println!("Config saved to: {}", config_path.display());
        config
    };
    let orchestrator = BuildOrchestrator::new(config);
    let report = orchestrator.run().await?;
    report.print();
    Ok(())
}
async fn handle_generate(output: PathBuf, auto: bool) -> Result<()> {
    println!("{}", "📝 Generating DDR Configuration".bright_cyan().bold());
    let config = if auto {
        auto_detect_config()?
    } else {
        generate_config(None, vec![], 16)?
    };
    let toml_str = toml::to_string_pretty(&config)?;
    fs::write(&output, toml_str)?;
    println!("✅ Configuration saved to: {}", output.display());
    println!("\nExample usage:");
    println!("  cm ddr build --use-config");
    println!("  cm ddr build -t x86_64-unknown-linux-musl -j 8");
    Ok(())
}
async fn handle_status(verbose: bool) -> Result<()> {
    println!("{}", "📊 DDR Build Status".bright_cyan().bold());
    let output = Command::new("docker")
        .args(
            &[
                "ps",
                "--filter",
                "label=ddr=true",
                "--format",
                "table {{.ID}}\t{{.Names}}\t{{.Status}}",
            ],
        )
        .output()?;
    if output.status.success() {
        let containers = String::from_utf8_lossy(&output.stdout);
        if !containers.trim().is_empty() {
            println!("\nActive DDR Containers:");
            println!("{}", containers);
        } else {
            println!("\nNo active DDR builds.");
        }
    }
    if verbose {
        let output = Command::new("docker")
            .args(
                &[
                    "images",
                    "--filter",
                    "label=ddr=true",
                    "--format",
                    "table {{.Repository}}\t{{.Tag}}\t{{.Size}}",
                ],
            )
            .output()?;
        if output.status.success() {
            let images = String::from_utf8_lossy(&output.stdout);
            if !images.trim().is_empty() {
                println!("\nDDR Images:");
                println!("{}", images);
            }
        }
    }
    Ok(())
}
async fn handle_clean(all: bool, project: Option<String>) -> Result<()> {
    println!("{}", "🧹 Cleaning DDR Artifacts".bright_cyan().bold());
    if all {
        println!("Removing all DDR containers...");
        Command::new("docker")
            .args(&["rm", "-f", "$(docker ps -aq --filter label=ddr=true)"])
            .output()?;
        println!("Removing all DDR images...");
        Command::new("docker")
            .args(&["rmi", "-f", "$(docker images -q --filter label=ddr=true)"])
            .output()?;
        println!("✅ All DDR artifacts cleaned.");
    } else if let Some(proj) = project {
        println!("Cleaning project: {}", proj);
        Command::new("docker")
            .args(
                &[
                    "rm",
                    "-f",
                    &format!("$(docker ps -aq --filter label=ddr.project={})", proj),
                ],
            )
            .output()?;
        println!("✅ Project {} cleaned.", proj);
    } else {
        println!("Specify --all or --project <name> to clean.");
    }
    Ok(())
}
async fn handle_validate(config_path: PathBuf) -> Result<()> {
    println!("{}", "🔍 Validating DDR Configuration".bright_cyan().bold());
    if !config_path.exists() {
        bail!("Config file not found: {}", config_path.display());
    }
    let content = fs::read_to_string(&config_path)?;
    match toml::from_str::<DdrConfig>(&content) {
        Ok(config) => {
            println!("✅ Configuration is valid!");
            println!("\nProject: {} v{}", config.project.name, config.project.version);
            println!("Targets: {}", config.targets.len());
            println!("Max Jobs: {}", config.parallel.max_jobs);
            let output = Command::new("docker")
                .args(&["version", "--format", "{{.Server.Version}}"])
                .output()?;
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("Docker: ✅ v{}", version.trim());
            } else {
                println!("Docker: ❌ Not available");
            }
        }
        Err(e) => {
            bail!("Configuration invalid: {}", e);
        }
    }
    Ok(())
}
fn generate_config(
    image: Option<String>,
    targets: Vec<String>,
    jobs: usize,
) -> Result<DdrConfig> {
    let cargo_toml = fs::read_to_string("Cargo.toml")?;
    let cargo: toml::Value = toml::from_str(&cargo_toml)?;
    let project_name = cargo["package"]["name"]
        .as_str()
        .unwrap_or("myproject")
        .to_string();
    let project_version = cargo["package"]["version"]
        .as_str()
        .unwrap_or("0.1.0")
        .to_string();
    let default_targets = if targets.is_empty() {
        vec![
            "x86_64-unknown-linux-musl".to_string(), "x86_64-unknown-linux-gnu"
            .to_string(), "x86_64-pc-windows-gnu".to_string(),
            "aarch64-unknown-linux-musl".to_string(),
        ]
    } else {
        targets
    };
    let mut target_configs = HashMap::new();
    for target in default_targets {
        let image = match target.as_str() {
            t if t.contains("musl") => "messense/rust-musl-cross:x86_64-musl",
            t if t.contains("windows") => "rust:latest",
            _ => image.as_deref().unwrap_or("rust:latest"),
        };
        target_configs
            .insert(
                target.clone(),
                TargetConfig {
                    triple: target.clone(),
                    image: image.to_string(),
                    dockerfile: None,
                    features: None,
                    rustflags: None,
                    linker: None,
                    strip: Some(true),
                    upx: Some(false),
                    test: Some(false),
                    bench: Some(false),
                    priority: None,
                },
            );
    }
    Ok(DdrConfig {
        project: ProjectConfig {
            name: project_name,
            version: project_version,
            workspace: None,
            cargo_toml: PathBuf::from("Cargo.toml"),
            src_dir: PathBuf::from("src"),
        },
        docker: DockerConfig {
            registry: None,
            build_args: None,
            network: None,
            volumes: None,
            env: None,
        },
        targets: target_configs,
        parallel: ParallelConfig {
            max_jobs: jobs,
            batch_size: None,
            timeout_minutes: Some(30),
            retry_failed: Some(1),
            fail_fast: Some(false),
        },
        cache: Some(CacheConfig {
            registry_cache: Some(true),
            cargo_cache: Some(PathBuf::from("~/.cargo")),
            sccache: Some(false),
            cache_from: None,
        }),
        artifacts: Some(ArtifactConfig {
            output_dir: PathBuf::from("target/ddr"),
            compress: Some(false),
            checksum: Some(true),
            manifest: Some(true),
        }),
    })
}
fn auto_detect_config() -> Result<DdrConfig> {
    println!("🔍 Auto-detecting build configuration...");
    let output = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()?;
    let installed_targets = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    println!("Found {} installed targets", installed_targets.len());
    let cpu_count = num_cpus::get();
    let recommended_jobs = (cpu_count / 2).max(1).min(16);
    println!("System CPUs: {}, Recommended jobs: {}", cpu_count, recommended_jobs);
    generate_config(None, installed_targets, recommended_jobs)
}