use indicatif::{ProgressBar, ProgressStyle as IndicatifStyle, MultiProgress};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use colored::*;
use anyhow::Result;
use crate::progress::tracker::{BuildTracker, BuildTrend};
pub enum ProgressStyle {
    Nautical,
    Classic,
    Minimal,
    Verbose,
}
pub struct BuildProgressBar {
    multi: MultiProgress,
    main_bar: ProgressBar,
    status_bar: Option<ProgressBar>,
    warning_counter: Arc<AtomicUsize>,
    error_counter: Arc<AtomicUsize>,
    tracker: Arc<Mutex<BuildTracker>>,
    style: ProgressStyle,
}
impl BuildProgressBar {
    pub fn new() -> Result<Self> {
        Self::with_style(ProgressStyle::Nautical)
    }
    pub fn with_style(style: ProgressStyle) -> Result<Self> {
        let multi = MultiProgress::new();
        let main_bar = multi.add(ProgressBar::new(100));
        main_bar.set_style(Self::get_indicatif_style(&style));
        let status_bar = if matches!(style, ProgressStyle::Verbose) {
            Some(multi.add(ProgressBar::new_spinner()))
        } else {
            None
        };
        Ok(Self {
            multi,
            main_bar,
            status_bar,
            warning_counter: Arc::new(AtomicUsize::new(0)),
            error_counter: Arc::new(AtomicUsize::new(0)),
            tracker: Arc::new(Mutex::new(BuildTracker::new()?)),
            style,
        })
    }
    fn get_indicatif_style(style: &ProgressStyle) -> IndicatifStyle {
        match style {
            ProgressStyle::Nautical => {
                IndicatifStyle::default_bar()
                    .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("⚓▬▬")
                    .tick_strings(
                        &[
                            "⚓ ",
                            "🌊 ",
                            "⛵ ",
                            "🚢 ",
                            "🏴‍☠️ ",
                            "🧭 ",
                            "⚡ ",
                            "🦜 ",
                            "🐙 ",
                            "🦈 ",
                            "🐚 ",
                            "🌊 ",
                            "⚓ ",
                        ],
                    )
            }
            ProgressStyle::Classic => {
                IndicatifStyle::default_bar()
                    .template(
                        "{spinner:.green} [{bar:40.green/white}] {pos}/{len} {msg}",
                    )
                    .unwrap()
                    .progress_chars("=>-")
            }
            ProgressStyle::Minimal => {
                IndicatifStyle::default_bar()
                    .template("{bar:50} {msg}")
                    .unwrap()
                    .progress_chars("█▓░")
            }
            ProgressStyle::Verbose => {
                IndicatifStyle::default_bar()
                    .template(
                        "{spinner:.cyan} {msg}\n[{bar:50.cyan/blue}] {pos}/{len} ({percent}%) ETA: {eta}",
                    )
                    .unwrap()
                    .progress_chars("=>-")
            }
        }
    }
    pub fn start_build(&self, command: &str) {
        let stats = self.tracker.lock().unwrap().get_stats();
        let message = match stats.trend {
            BuildTrend::Improving => {
                format!(
                    "📈 Building... | Success Rate: {:.1}% (improving!) | Last: {} ⚠️  {} ❌",
                    stats.success_rate * 100.0, stats.avg_warnings as usize, stats
                    .avg_errors as usize
                )
            }
            BuildTrend::Degrading => {
                format!(
                    "📉 Building... | Success Rate: {:.1}% (needs attention) | Last: {} ⚠️  {} ❌",
                    stats.success_rate * 100.0, stats.avg_warnings as usize, stats
                    .avg_errors as usize
                )
            }
            BuildTrend::Stable => {
                format!(
                    "🔨 Building... | Success Rate: {:.1}% | Last: {} ⚠️  {} ❌",
                    stats.success_rate * 100.0, stats.avg_warnings as usize, stats
                    .avg_errors as usize
                )
            }
        };
        self.main_bar.set_message(message);
        self.main_bar.enable_steady_tick(Duration::from_millis(100));
        if let Some(status) = &self.status_bar {
            status.set_message(format!("Executing: {}", command.yellow()));
            status.enable_steady_tick(Duration::from_millis(80));
        }
    }
    pub fn update_progress(&self, current: u64, total: u64) {
        self.main_bar.set_length(total);
        self.main_bar.set_position(current);
    }
    pub fn update_counters(&self, warnings: usize, errors: usize) {
        self.warning_counter.store(warnings, Ordering::Relaxed);
        self.error_counter.store(errors, Ordering::Relaxed);
        let msg = if errors > 0 {
            format!(
                "❌ {} errors | ⚠️  {} warnings", errors.to_string().red(), warnings
                .to_string().yellow()
            )
                .red()
                .to_string()
        } else if warnings > 0 {
            format!("⚠️  {} warnings", warnings).yellow().to_string()
        } else {
            "✨ Clean build!".green().to_string()
        };
        self.main_bar.set_message(msg);
    }
    pub fn log_message(&self, message: &str) {
        if let Some(status) = &self.status_bar {
            status.set_message(message.to_string());
        }
    }
    pub fn finish(&self, success: bool) {
        if success {
            let warnings = self.warning_counter.load(Ordering::Relaxed);
            let message = if warnings > 0 {
                format!("✅ Build successful with {} warnings", warnings)
                    .yellow()
                    .to_string()
            } else {
                "✅ Build successful! 🎉".green().bold().to_string()
            };
            self.main_bar.finish_with_message(message);
        } else {
            let errors = self.error_counter.load(Ordering::Relaxed);
            self.main_bar
                .finish_with_message(
                    format!("❌ Build failed with {} errors", errors)
                        .red()
                        .bold()
                        .to_string(),
                );
        }
        if let Some(status) = &self.status_bar {
            status.finish_and_clear();
        }
    }
    pub fn get_warning_count(&self) -> usize {
        self.warning_counter.load(Ordering::Relaxed)
    }
    pub fn get_error_count(&self) -> usize {
        self.error_counter.load(Ordering::Relaxed)
    }
    pub fn show_summary(&self) {
        let stats = self.tracker.lock().unwrap().get_stats();
        println!("\n{}", "═".repeat(60).cyan());
        println!("{}", "📊 Build Summary".cyan().bold());
        println!("{}", "═".repeat(60).cyan());
        println!("Total Builds: {}", stats.total_builds.to_string().yellow());
        println!("Success Rate: {:.1}%", (stats.success_rate * 100.0));
        println!("Average Warnings: {:.1}", stats.avg_warnings);
        println!("Average Errors: {:.1}", stats.avg_errors);
        if let Some(last_success) = &stats.last_success {
            println!(
                "Last Success: {} ({}ms)", last_success.timestamp.format("%H:%M:%S"),
                last_success.duration_ms
            );
        }
        if let Some(last_failure) = &stats.last_failure {
            println!(
                "Last Failure: {} ({} errors)", last_failure.timestamp
                .format("%H:%M:%S"), last_failure.errors
            );
        }
        match stats.trend {
            BuildTrend::Improving => {
                println!("Trend: {} Improving! Keep it up! 🚀", "📈".green())
            }
            BuildTrend::Stable => println!("Trend: {} Stable", "➡️".yellow()),
            BuildTrend::Degrading => {
                println!("Trend: {} Degrading - needs attention", "📉".red())
            }
        }
        println!("{}", "═".repeat(60).cyan());
    }
}