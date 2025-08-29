use crate::checklist;
use crate::history;
use crate::parser::{self, MessageData, ParsedError, ParsedWarning};
use crate::tide::{TideCharts, BuildMetrics};
use crate::captain::license;
use colored::*;
use anyhow::{Result, Context};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use chrono::Utc;
use std::collections::{HashMap, HashSet, VecDeque};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDeduplicator {
    seen_fingerprints: HashMap<String, ErrorGroup>,
    similarity_threshold: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorGroup {
    primary_error: ParsedError,
    variations: Vec<ParsedError>,
    count: usize,
    first_seen: String,
    locations: HashSet<String>,
}
impl ErrorDeduplicator {
    pub fn new() -> Self {
        Self {
            seen_fingerprints: HashMap::new(),
            similarity_threshold: 0.8,
        }
    }
    pub fn fingerprint(&self, error: &ParsedError) -> String {
        let mut hasher = Sha256::new();
        let normalized = self.normalize_error_message(&error.message);
        hasher.update(normalized.as_bytes());
        if !error.file.is_empty() {
            hasher.update(error.file.as_bytes());
            if error.line > 0 {
                hasher.update((error.line / 10).to_string().as_bytes());
            }
        }
        format!("{:x}", hasher.finalize())
    }
    fn normalize_error_message(&self, msg: &str) -> String {
        msg.split_whitespace()
            .map(|word| {
                if word.starts_with('`') && word.ends_with('`') {
                    "`<identifier>`"
                } else {
                    word
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub fn process_errors(&mut self, errors: &[ParsedError]) -> Vec<ErrorGroup> {
        for error in errors {
            let fingerprint = self.fingerprint(error);
            self.seen_fingerprints
                .entry(fingerprint)
                .or_insert_with(|| ErrorGroup {
                    primary_error: error.clone(),
                    variations: Vec::new(),
                    count: 0,
                    first_seen: Utc::now().to_rfc3339(),
                    locations: HashSet::new(),
                })
                .add_variation(error);
        }
        let mut groups: Vec<_> = self.seen_fingerprints.values().cloned().collect();
        groups.sort_by(|a, b| b.count.cmp(&a.count));
        groups
    }
}
impl ErrorGroup {
    pub fn add_variation(&mut self, error: &ParsedError) {
        self.variations.push(error.clone());
        self.count += 1;
        if !error.file.is_empty() {
            self.locations.insert(format!("{}:{}", error.file, error.line));
        }
    }
}
#[derive(Debug, Clone)]
pub struct ErrorPrioritizer {
    weights: PriorityWeights,
}
#[derive(Debug, Clone)]
pub struct PriorityWeights {
    never_seen_before: f32,
    blocking_compilation: f32,
    has_quick_fix: f32,
    frequently_ignored: f32,
    in_dependency: f32,
    test_only: f32,
}
impl Default for PriorityWeights {
    fn default() -> Self {
        Self {
            never_seen_before: 10.0,
            blocking_compilation: 8.0,
            has_quick_fix: -2.0,
            frequently_ignored: -5.0,
            in_dependency: -3.0,
            test_only: -1.0,
        }
    }
}
impl ErrorPrioritizer {
    pub fn new() -> Self {
        Self {
            weights: PriorityWeights::default(),
        }
    }
    pub fn sort_errors(&self, errors: Vec<ParsedError>) -> Vec<ParsedError> {
        let mut scored_errors: Vec<(ParsedError, f32)> = errors
            .into_iter()
            .map(|error| {
                let mut score = 5.0;
                score += self.weights.never_seen_before;
                if self.has_known_fix(&error) {
                    score += self.weights.has_quick_fix;
                }
                if error.file.contains("/dependencies/") {
                    score += self.weights.in_dependency;
                }
                if error.file.contains("/tests/") || error.file.contains("_test.rs") {
                    score += self.weights.test_only;
                }
                (error, score)
            })
            .collect();
        scored_errors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored_errors.into_iter().map(|(e, _)| e).collect()
    }
    fn has_known_fix(&self, error: &ParsedError) -> bool {
        false
    }
}
#[derive(Debug, Clone)]
pub struct BuildCoach {
    tips: Vec<CoachingTip>,
    shown_tips: HashSet<String>,
}
#[derive(Debug, Clone)]
pub struct CoachingTip {
    id: String,
    condition: BuildCondition,
    message: String,
    priority: u8,
}
#[derive(Debug, Clone)]
pub enum BuildCondition {
    SlowBuild(Duration),
    ManyWarnings(usize),
    RecurringErrors,
    LargeErrorCount(usize),
}
impl BuildCoach {
    pub fn new() -> Self {
        let mut tips = Vec::new();
        tips.push(CoachingTip {
            id: "slow_build".to_string(),
            condition: BuildCondition::SlowBuild(Duration::from_secs(30)),
            message: "💡 Long build? Try `cm optimize aggressive` for faster builds"
                .to_string(),
            priority: 5,
        });
        tips.push(CoachingTip {
            id: "many_warnings".to_string(),
            condition: BuildCondition::ManyWarnings(20),
            message: "💡 Many warnings? Use `cm mutiny allow-warnings` temporarily"
                .to_string(),
            priority: 3,
        });
        tips.push(CoachingTip {
            id: "recurring_error".to_string(),
            condition: BuildCondition::RecurringErrors,
            message: "💡 Recurring error? Try `cm wtf er` for AI assistance"
                .to_string(),
            priority: 8,
        });
        tips.push(CoachingTip {
            id: "many_errors".to_string(),
            condition: BuildCondition::LargeErrorCount(10),
            message: "💡 Many errors? Focus on the first few - they often cascade"
                .to_string(),
            priority: 6,
        });
        Self {
            tips,
            shown_tips: HashSet::new(),
        }
    }
    pub fn check_and_show_tip(&mut self, context: &BuildContext) -> Option<String> {
        for tip in &self.tips {
            if !self.shown_tips.contains(&tip.id) && tip.condition.matches(context) {
                self.shown_tips.insert(tip.id.clone());
                return Some(tip.message.clone());
            }
        }
        None
    }
}
impl BuildCondition {
    pub fn matches(&self, context: &BuildContext) -> bool {
        match self {
            BuildCondition::SlowBuild(duration) => context.elapsed > *duration,
            BuildCondition::ManyWarnings(count) => context.warning_count > *count,
            BuildCondition::RecurringErrors => context.has_recurring_errors,
            BuildCondition::LargeErrorCount(count) => context.error_count > *count,
        }
    }
}
#[derive(Debug)]
pub struct BuildContext {
    pub elapsed: Duration,
    pub warning_count: usize,
    pub error_count: usize,
    pub has_recurring_errors: bool,
}
fn process_and_display_errors(errors: &[ParsedError]) {
    if errors.is_empty() {
        return;
    }
    let mut deduplicator = ErrorDeduplicator::new();
    let groups = deduplicator.process_errors(errors);
    if !groups.is_empty() {
        println!(
            "\n{}", format!("🔴 {} Unique Error Patterns:", groups.len()) .red().bold()
        );
        for (i, group) in groups.iter().take(5).enumerate() {
            println!(
                "  {}. {} ({}x across {} locations)", i + 1, group.primary_error.message,
                group.count, group.locations.len()
            );
            if group.variations.len() > 1 {
                println!(
                    "     {} Similar variations grouped", group.variations.len()
                    .to_string().dimmed()
                );
            }
        }
    }
}
pub fn run_cargo_passthrough(args: &[&str]) {
    let cargo_path = std::env::var("CARGO_BIN_PATH")
        .unwrap_or_else(|_| "/root/.cargo/bin/cargo".to_string());
    let status = Command::new(&cargo_path)
        .args(args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to start cargo: {}", e);
            std::process::exit(1);
        });
    std::process::exit(status.code().unwrap_or(1));
}
const NAUTICAL_MESSAGES: &[&str] = &[
    "[ANCHOR] Dropping anchor and securing position...",
    "[WAVE] Riding the waves with steady resolve...",
    "[PIRATE] Hoisting the Jolly Roger - compilation begins! [SWORD]",
    "[MAP] Charting course through dependency seas...",
    "[SAIL] Catching wind in our dependency sails...",
    "[SHIP] Setting sail across the Rust seas...",
    "[COMPASS] Navigating treacherous compilation waters...",
    "[SUNRISE] Chasing horizons of clean builds...",
    "[HAMMER] Forging dependencies in the shipyard...",
    "[GEAR] Machining precision components...",
    "[BOLT] Tightening bolts in the engine room...",
    "[WRENCH] Calibrating the build compass...",
    "[RULER] Measuring twice, compiling once...",
    "[SCOPE] Inspecting code quality under magnification...",
    "[FLASK] Distilling pure Rust essence...",
    "[TEST] Testing the waters before deep diving...",
    "📦 Loading cargo containers with care...",
    "🚛 Hauling dependencies across the digital dock...",
    "🏗️ Constructing the foundation of your project...",
    "🧱 Laying bricks of reliable code...",
    "🏭 Manufacturing robust binaries...",
    "📋 Checking manifest against the cargo log...",
    "🔍 Scanning for hidden treasures in the code...",
    "🧹 Sweeping the deck of compilation artifacts...",
    "⚡ Full speed ahead - compiling at flank speed! ⚡",
    "🎯 Setting course for build success...",
    "🌟 Following the North Star of clean code...",
    "🏆 Battling compilation dragons...",
    "🛡️ Shielding against compilation errors...",
    "🎪 Performing the great cargo circus act...",
    "🎭 Wearing multiple compilation hats...",
    "🎪 Juggling dependencies like a master performer...",
    "🌊 Sailing through calm compilation seas...",
    "⛈️ Weathering the storm of complex dependencies...",
    "🌪️ Surfing the waves of async compilation...",
    "🌈 Riding the rainbow after the storm...",
    "🌊 Dancing with the tides of build progress...",
    "🌅 Sunset approaches - build almost complete...",
    "🌄 Dawn breaks - new build cycle begins...",
    "🌠 Shooting stars guide our compilation path...",
    "👥 Manning the compilation stations...",
    "🏴‍☠️ Crew chanting sea shanties of success...",
    "🧑‍⚓ First mate checking the build log...",
    "👨‍🍳 Cook preparing a feast of fresh binaries...",
    "🧑‍🚀 Navigator plotting course through error logs...",
    "👨‍🔧 Engineer fine-tuning the compilation engine...",
    "🧑‍🎨 Artist painting the canvas of clean code...",
    "👩‍⚖️ Judge reviewing code quality standards...",
    "🏴‍☠️ Searching for buried compilation treasures...",
    "🗝️ Unlocking the secrets of dependency resolution...",
    "💎 Polishing the gems of optimized code...",
    "🗺️ Following the treasure map of build instructions...",
    "🔮 Crystal ball shows successful compilation...",
    "🧙‍♂️ Wizard casting spells of optimization...",
    "🦄 Unicorn blessing the codebase...",
    "🐉 Dragon guarding the gates of compilation success...",
    "🔧 Twisting the knobs of optimization...",
    "⚖️ Balancing the scales of performance...",
    "🔄 Spinning the wheels of progress...",
    "📊 Graphing the peaks of build performance...",
    "🎵 Orchestrating the symphony of compilation...",
    "🎭 Directing the play of parallel compilation...",
    "🎪 Conducting the circus of crate dependencies...",
    "🎨 Painting the masterpiece of working binaries...",
    "🦀 Crab walking through memory safety checks...",
    "🦀 Pinning ownership to the compilation board...",
    "🦀 Borrowing references from the lending library...",
    "🦀 Sending values across the borrow checker...",
    "🦀 Moving types through the ownership maze...",
    "🦀 Deriving traits from the trait workshop...",
    "🦀 Implementing interfaces in the code factory...",
    "🦀 Matching patterns in the pattern matching parlor...",
    "🎪 Clown car of dependencies arriving...",
    "🤖 Robot army assembling your binaries...",
    "🚀 Spaceship preparing for launch sequence...",
    "🧠 Brain computing optimal compilation path...",
    "🎯 Target acquired - building with precision...",
    "🧩 Piecing together the puzzle of dependencies...",
    "🎪 Big top compilation show in progress...",
    "🎪 Tent of dependencies being raised...",
    "☀️ Sunny compilation day ahead...",
    "🌙 Night shift compilation crew reporting...",
    "❄️ Cool compilation in progress...",
    "🔥 Hot compilation action heating up...",
    "🌪️ Tornado of dependencies spinning up...",
    "🌈 Rainbow compilation bridge forming...",
    "⭐ Starry night compilation under way...",
    "🌌 Galactic compilation sequence initiated...",
];
const BUILD_STAGES: &[&str] = &[
    "🔍 Analyzing dependencies in the code harbor...",
    "📦 Downloading crates from the digital dockyard...",
    "🔨 Compiling dependencies in the shipyard forge...",
    "⚙️ Building project with precision engineering...",
    "🧪 Running tests through quality control gauntlet...",
    "📋 Generating documentation for future explorers...",
    "🚀 Finalizing build - preparing for launch sequence...",
    "🎯 Calibrating build targets and cross-checking manifests...",
    "🔬 Inspecting binaries under the quality microscope...",
    "📊 Generating build metrics and performance reports...",
    "🧹 Sweeping up compilation artifacts and loose ends...",
    "🏆 Polishing the final executable to a mirror shine...",
    "🚀 Loading binary into launch tube - ready for deployment...",
];
pub fn run_cargo_with_display(args: &[&str]) {
    let start_time = Instant::now();
    let mut error_deduplicator = ErrorDeduplicator::new();
    let error_prioritizer = ErrorPrioritizer::new();
    let mut build_coach = BuildCoach::new();
    let mut child = Command::new("cargo")
        .args(args)
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("Failed to start cargo: {}", e);
            std::process::exit(1);
        });
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stdout);
    let err_reader = BufReader::new(stderr);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut artifacts = Vec::new();
    let mut build_scripts = Vec::new();
    let error_count = Arc::new(AtomicUsize::new(0));
    let warning_count = Arc::new(AtomicUsize::new(0));
    let artifact_count = Arc::new(AtomicUsize::new(0));
    let multi_progress = MultiProgress::new();
    let main_pb = create_main_progress_bar();
    let main_pb = multi_progress.add(main_pb);
    let status_pb = create_status_bar();
    let status_pb = multi_progress.add(status_pb);
    let file_pb = create_file_counter_bar();
    let file_pb = multi_progress.add(file_pb);
    main_pb.set_message(format!("🚢 Starting cargo {}", args.join(" ")));
    status_pb.set_message("⏳ Initializing...");
    file_pb.set_message("📁 0 files processed");
    let mut message_index = 0;
    let mut stage_index = 0;
    let mut tick_count = 0;
    let mut last_stage_change = Instant::now();
    if !NAUTICAL_MESSAGES.is_empty() && message_index >= NAUTICAL_MESSAGES.len() {
        message_index = 0;
    }
    if !BUILD_STAGES.is_empty() && stage_index >= BUILD_STAGES.len() {
        stage_index = 0;
    }
    let err_handle = thread::spawn(move || {
        let reader = BufReader::new(err_reader);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("{}", line);
            }
        }
    });
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(msg) = parser::parse_cargo_message(&line) {
                match msg.data {
                    MessageData::CompilerMessage(cm) => {
                        match cm.message.level.as_str() {
                            "error" => {
                                let parsed_error = parser::format_error(&cm.message);
                                errors.push(parsed_error.clone());
                                error_count.store(errors.len(), Ordering::Relaxed);
                                status_pb
                                    .set_message(
                                        format!(
                                            "🔴 {} errors, ⚠️ {} warnings", error_count
                                            .load(Ordering::Relaxed), warning_count
                                            .load(Ordering::Relaxed)
                                        ),
                                    );
                                let fingerprint = error_deduplicator
                                    .fingerprint(&parsed_error);
                                error_deduplicator.process_errors(&[parsed_error]);
                            }
                            "warning" => {
                                let parsed_warning = parser::format_warning(&cm.message);
                                warnings.push(parsed_warning.clone());
                                warning_count.store(warnings.len(), Ordering::Relaxed);
                                status_pb
                                    .set_message(
                                        format!(
                                            "🔴 {} errors, ⚠️ {} warnings", error_count
                                            .load(Ordering::Relaxed), warning_count
                                            .load(Ordering::Relaxed)
                                        ),
                                    );
                            }
                            _ => {}
                        }
                    }
                    MessageData::BuildScriptExecuted(bs) => {
                        build_scripts.push(bs);
                        artifact_count
                            .store(
                                artifact_count.load(Ordering::Relaxed) + 1,
                                Ordering::Relaxed,
                            );
                        file_pb
                            .set_message(
                                format!(
                                    "📁 {} files, 🔨 {} build scripts", artifact_count
                                    .load(Ordering::Relaxed), build_scripts.len()
                                ),
                            );
                    }
                    MessageData::CompilerArtifact(ca) => {
                        artifacts.push(ca);
                        artifact_count.store(artifacts.len(), Ordering::Relaxed);
                        file_pb
                            .set_message(
                                format!(
                                    "📁 {} files, 🔨 {} build scripts", artifact_count
                                    .load(Ordering::Relaxed), build_scripts.len()
                                ),
                            );
                    }
                    _ => {}
                }
                tick_count += 1;
                if tick_count > 1_000_000 {
                    tick_count = 0;
                }
                if tick_count % 3 == 0 && !NAUTICAL_MESSAGES.is_empty() {
                    message_index = (message_index + 1) % NAUTICAL_MESSAGES.len();
                    if message_index >= NAUTICAL_MESSAGES.len() {
                        message_index = 0;
                    }
                    if let Some(message) = NAUTICAL_MESSAGES.get(message_index) {
                        main_pb.set_prefix(message.to_string());
                    }
                }
                if last_stage_change.elapsed() > Duration::from_secs(2)
                    && !BUILD_STAGES.is_empty()
                {
                    stage_index = (stage_index + 1) % BUILD_STAGES.len();
                    if stage_index >= BUILD_STAGES.len() {
                        stage_index = 0;
                    }
                    if let Some(stage) = BUILD_STAGES.get(stage_index) {
                        status_pb.set_message(stage.to_string());
                    }
                    last_stage_change = Instant::now();
                }
                main_pb.tick();
                status_pb.tick();
                file_pb.tick();
            }
        }
    }
    let elapsed = start_time.elapsed();
    main_pb
        .finish_with_message(
            format!(
                "🚢 Cargo {} completed in {:.1}s", args.join(" "), elapsed
                .as_secs_f32()
            ),
        );
    status_pb.finish_with_message("✅ Build finished");
    file_pb
        .finish_with_message(
            format!("📁 {} files processed", artifact_count.load(Ordering::Relaxed)),
        );
    let _ = err_handle.join();
    let status = child.wait().unwrap();
    let has_recurring_errors = !errors.is_empty()
        && error_count.load(Ordering::Relaxed) > 1;
    let build_context = BuildContext {
        elapsed,
        warning_count: warnings.len(),
        error_count: errors.len(),
        has_recurring_errors,
    };
    if let Some(tip) = build_coach.check_and_show_tip(&build_context) {
        println!("\n{}", tip.cyan());
    }
    save_results(&errors, &warnings, &artifacts, &build_scripts, args);
    record_build_metrics(args, elapsed, errors.len(), warnings.len(), status.success());
    display_summary(
        &errors,
        &warnings,
        &artifacts,
        &build_scripts,
        status.success(),
        elapsed,
    );
    if !errors.is_empty() {
        let prioritized_errors = error_prioritizer.sort_errors(errors.clone());
        process_and_display_errors(&prioritized_errors);
    }
    if !errors.is_empty() || !warnings.is_empty() {
        checklist::generate_checklist(&errors, &warnings);
        println!("\n📋 Run {} to see your checklist", "cm checklist".yellow());
    }
    display_view_options(&errors, &warnings, &artifacts, &build_scripts);
}
fn create_main_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{prefix:.cyan} {spinner:.green} {msg}")
            .unwrap()
            .tick_chars("|-\\|/-"),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}
fn create_status_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_chars("...oooOOO"),
    );
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}
fn create_file_counter_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.yellow} {msg}")
            .unwrap()
            .tick_chars("123456789"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
fn save_results(
    errors: &[ParsedError],
    warnings: &[ParsedWarning],
    artifacts: &[parser::CompilerArtifact],
    build_scripts: &[parser::BuildScriptExecuted],
    args: &[&str],
) {
    let shipwreck = dirs::home_dir().unwrap().join(".shipwreck");
    fs::create_dir_all(&shipwreck).unwrap();
    let error_file = shipwreck.join("errors").join("latest.txt");
    fs::create_dir_all(error_file.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&error_file).unwrap();
    for error in errors {
        writeln!(f, "{}", error).unwrap();
    }
    let warning_file = shipwreck.join("warnings").join("latest.txt");
    fs::create_dir_all(warning_file.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&warning_file).unwrap();
    for warning in warnings {
        writeln!(f, "{}", warning).unwrap();
    }
    let artifact_file = shipwreck.join("artifacts").join("latest.txt");
    fs::create_dir_all(artifact_file.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&artifact_file).unwrap();
    for artifact in artifacts {
        writeln!(f, "📦 {} -> {}", artifact.target.name, artifact.filenames.join(", "))
            .unwrap();
    }
    let script_file = shipwreck.join("scripts").join("latest.txt");
    fs::create_dir_all(script_file.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&script_file).unwrap();
    for script in build_scripts {
        writeln!(
            f, "🔨 {} -> libs: {}, paths: {}, cfgs: {}", script.package_id, script
            .linked_libs.len(), script.linked_paths.len(), script.cfgs.len()
        )
            .unwrap();
    }
    history::save_to_history(args.join(" "), errors.to_vec(), warnings.to_vec());
}
fn display_summary(
    errors: &[ParsedError],
    warnings: &[ParsedWarning],
    artifacts: &[parser::CompilerArtifact],
    build_scripts: &[parser::BuildScriptExecuted],
    success: bool,
    elapsed: Duration,
) {
    println!("\n{}", "═".repeat(60).blue());
    if success && errors.is_empty() {
        println!("{}", "✅ Build Successful!".green().bold());
    } else {
        println!("{}", "❌ Build Failed!".red().bold());
    }
    println!("⏱️  Build time: {:.1}s", elapsed.as_secs_f32());
    println!("📁 Files generated: {}", artifacts.len());
    println!("🔨 Build scripts: {}", build_scripts.len());
    if !errors.is_empty() {
        println!("\n{}", format!("🔴 {} Error(s):", errors.len()) .red().bold());
        for (i, error) in errors.iter().take(3).enumerate() {
            println!("  {}. {}", i + 1, error);
        }
        if errors.len() > 3 {
            println!("  ... and {} more", errors.len() - 3);
        }
    }
    if !warnings.is_empty() {
        println!(
            "\n{}", format!("⚠️  {} Warning(s):", warnings.len()) .yellow().bold()
        );
        for (i, warning) in warnings.iter().take(3).enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
        if warnings.len() > 3 {
            println!("  ... and {} more", warnings.len() - 3);
        }
    }
    println!("{}", "═".repeat(60).blue());
}
fn display_view_options(
    errors: &[ParsedError],
    warnings: &[ParsedWarning],
    artifacts: &[parser::CompilerArtifact],
    build_scripts: &[parser::BuildScriptExecuted],
) {
    println!("\n🔍 View Options:");
    println!("  {} - View all errors and warnings", "cm view errors".cyan());
    println!("  {} - View generated files and locations", "cm view artifacts".cyan());
    println!("  {} - View build script outputs", "cm view scripts".cyan());
    println!("  {} - View detailed build history", "cm view history".cyan());
    println!("  {} - View checklist and fixes", "cm view checklist".cyan());
    println!("  {} - View all results in one place", "cm view all".cyan());
    if !errors.is_empty() || !warnings.is_empty() {
        println!("  {} - Quick view of latest issues", "cm view latest".cyan());
    }
    println!("  {} - Open results in file explorer", "cm view open".cyan());
}
fn record_build_metrics(
    args: &[&str],
    elapsed: Duration,
    error_count: usize,
    warning_count: usize,
    success: bool,
) {
    if let Ok(mut tide) = TideCharts::new() {
        let command = format!("cargo {}", args.join(" "));
        let profile = determine_profile(args);
        let features = extract_features(args);
        let dependencies_compiled = get_dependencies_compiled();
        let crate_units_compiled = get_crate_units_compiled();
        let metrics = BuildMetrics {
            timestamp: Utc::now(),
            command,
            duration_seconds: elapsed.as_secs_f64(),
            success,
            error_count,
            warning_count,
            incremental: args.contains(&"--incremental") || args.contains(&"-i"),
            profile,
            features,
            dependencies_compiled,
            crate_units_compiled,
            memory_peak_mb: None,
            cpu_usage_percent: None,
        };
        if let Err(e) = tide.record_build(metrics) {
            eprintln!("⚠️  Failed to record build metrics: {}", e);
        }
    }
}
fn determine_profile(args: &[&str]) -> String {
    if args.contains(&"--release") {
        "release".to_string()
    } else if args.contains(&"--debug") {
        "debug".to_string()
    } else {
        for (i, arg) in args.iter().enumerate() {
            if *arg == "--profile" && i + 1 < args.len() {
                return args[i + 1].to_string();
            }
        }
        "debug".to_string()
    }
}
fn extract_features(args: &[&str]) -> Vec<String> {
    let mut features = Vec::new();
    let mut found_features = false;
    for (i, arg) in args.iter().enumerate() {
        if *arg == "--features" && i + 1 < args.len() {
            found_features = true;
            features = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
            break;
        } else if *arg == "--all-features" {
            features.push("all-features".to_string());
            break;
        } else if *arg == "--no-default-features" {
            features.push("no-default-features".to_string());
        }
    }
    if !found_features && !args.contains(&"--no-default-features")
        && !args.contains(&"--all-features")
    {
        features.push("default".to_string());
    }
    features
}
fn get_dependencies_compiled() -> usize {
    match Command::new("cargo").args(&["metadata", "--format-version", "1"]).output() {
        Ok(output) if output.status.success() => {
            if let Ok(metadata) = serde_json::from_slice::<
                serde_json::Value,
            >(&output.stdout) {
                if let Some(packages) = metadata
                    .get("packages")
                    .and_then(|p| p.as_array())
                {
                    if let Some(root) = metadata.get("root").and_then(|r| r.get("name"))
                    {
                        let root_name = root.as_str().unwrap_or("");
                        return packages
                            .iter()
                            .filter(|pkg| {
                                pkg.get("name")
                                    .and_then(|n| n.as_str())
                                    .map(|name| name != root_name)
                                    .unwrap_or(false)
                            })
                            .count();
                    }
                }
            }
        }
        _ => {}
    }
    0
}
fn get_crate_units_compiled() -> usize {
    0
}
pub fn check_first_mate_monitor(command: &str) -> Result<bool, anyhow::Error> {
    println!(
        "🥽 First mate monitoring command '{}' - all hands report!", command.cyan()
    );
    let license_manager = license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ First mate reports: Command '{}' cleared for action!", command
                .green()
            );
            println!("   🥽 All crew stations manned - ready to execute!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  First mate's log: Command ration exceeded!");
                println!("   🥽 Resupply at: https://cargo.do/checkout");
                println!("   🥽 Upgrade to unlimited command rations");
            } else if e.to_string().contains("License not found") {
                println!("❌ First mate reports: No command authority papers!");
                println!("   🥽 Commission with 'cm register <key>'");
            } else {
                println!(
                    "❌ First mate emergency: Command check failed: {}", e.to_string()
                    .red()
                );
                println!("   🥽 Secure all stations and alert the captain");
            }
            Ok(false)
        }
    }
}