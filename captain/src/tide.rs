use anyhow::{Context, Result};
use chrono::{DateTime, Timelike, Utc};
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend, layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    widgets::{
        Axis, BarChart, Block, Borders, Chart, Dataset, Gauge, List, ListItem, Paragraph,
        Sparkline, Tabs,
    },
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use crate::captain::license;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildMetrics {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub duration_seconds: f64,
    pub success: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub incremental: bool,
    pub profile: String,
    pub features: Vec<String>,
    pub dependencies_compiled: usize,
    pub crate_units_compiled: usize,
    pub memory_peak_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyMetrics {
    pub name: String,
    pub version: String,
    pub compile_time_seconds: f64,
    pub size_bytes: u64,
    pub features: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TideData {
    pub builds: Vec<BuildMetrics>,
    pub dependencies: HashMap<String, DependencyMetrics>,
    pub daily_summary: HashMap<String, DailySummary>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailySummary {
    pub date: String,
    pub total_builds: usize,
    pub successful_builds: usize,
    pub failed_builds: usize,
    pub total_time_seconds: f64,
    pub avg_build_time: f64,
    pub total_errors: usize,
    pub total_warnings: usize,
}
pub struct TideCharts {
    data: TideData,
    data_file: PathBuf,
    selected_tab: usize,
}
impl TideCharts {
    pub fn new() -> Result<Self> {
        let data_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("tide_data.json");
        let data = if data_file.exists() {
            let content = fs::read_to_string(&data_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            TideData::default()
        };
        Ok(Self {
            data,
            data_file,
            selected_tab: 0,
        })
    }
    pub fn record_build(&mut self, metrics: BuildMetrics) -> Result<()> {
        self.data.builds.push(metrics.clone());
        let date = metrics.timestamp.date_naive().to_string();
        let summary = self
            .data
            .daily_summary
            .entry(date.clone())
            .or_insert(DailySummary {
                date,
                total_builds: 0,
                successful_builds: 0,
                failed_builds: 0,
                total_time_seconds: 0.0,
                avg_build_time: 0.0,
                total_errors: 0,
                total_warnings: 0,
            });
        summary.total_builds += 1;
        if metrics.success {
            summary.successful_builds += 1;
        } else {
            summary.failed_builds += 1;
        }
        summary.total_time_seconds += metrics.duration_seconds;
        summary.avg_build_time = summary.total_time_seconds
            / summary.total_builds as f64;
        summary.total_errors += metrics.error_count;
        summary.total_warnings += metrics.warning_count;
        if self.data.builds.len() > 10000 {
            self.data.builds = self
                .data
                .builds[self.data.builds.len() - 10000..]
                .to_vec();
        }
        self.save()?;
        Ok(())
    }
    pub fn analyze_dependencies(&mut self) -> Result<()> {
        println!("🔍 Analyzing dependency compile times...");
        let output = Command::new("cargo").args(&["build", "--timings"]).output()?;
        if output.status.success() {
            println!(
                "✅ Timing data collected. Check target/cargo-timings/ for detailed report."
            );
        }
        let metadata = cargo_metadata::MetadataCommand::new().exec()?;
        for package in metadata.packages {
            if package.source.is_some() {
                let dep_metrics = DependencyMetrics {
                    name: package.name.clone(),
                    version: package.version.to_string(),
                    compile_time_seconds: 0.0,
                    size_bytes: 0,
                    features: package.features.keys().cloned().collect(),
                };
                self.data.dependencies.insert(package.name, dep_metrics);
            }
        }
        self.save()?;
        Ok(())
    }
    pub fn show_interactive(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let res = self.run_interactive(&mut terminal);
        disable_raw_mode()?;
        terminal.backend_mut().execute(LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        res
    }
    fn run_interactive<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Tab => {
                            self.selected_tab = (self.selected_tab + 1) % 4;
                        }
                        KeyCode::BackTab => {
                            self.selected_tab = if self.selected_tab > 0 {
                                self.selected_tab - 1
                            } else {
                                3
                            };
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
    fn ui(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.size());
        let titles = vec!["Overview", "Performance", "Dependencies", "Trends"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("🌊 Tide Charts"))
            .highlight_style(
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )
            .select(self.selected_tab);
        frame.render_widget(tabs, chunks[0]);
        match self.selected_tab {
            0 => self.render_overview(frame, chunks[1]),
            1 => self.render_performance(frame, chunks[1]),
            2 => self.render_dependencies(frame, chunks[1]),
            3 => self.render_trends(frame, chunks[1]),
            _ => {}
        }
        let help = Paragraph::new("Press Tab to switch views | q to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }
    fn render_overview(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(5),
            ])
            .split(area);
        let recent_builds = self.data.builds.iter().rev().take(50).collect::<Vec<_>>();
        let success_rate = if !recent_builds.is_empty() {
            let successful = recent_builds.iter().filter(|b| b.success).count();
            (successful as f64 / recent_builds.len() as f64) * 100.0
        } else {
            0.0
        };
        let stats = vec![
            format!("Total Builds: {}", self.data.builds.len()),
            format!("Success Rate: {:.1}%", success_rate),
            format!("Avg Build Time: {:.2}s", self.get_avg_build_time()),
            format!("Dependencies: {}", self.data.dependencies.len()),
        ];
        let stats_widget = Paragraph::new(stats.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("📊 Statistics"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(stats_widget, chunks[0]);
        let gauge = Gauge::default()
            .block(
                Block::default().borders(Borders::ALL).title("🎯 Build Success Rate"),
            )
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .percent(success_rate as u16)
            .label(format!("{:.1}%", success_rate));
        frame.render_widget(gauge, chunks[1]);
        let sparkline_data: Vec<u64> = recent_builds
            .iter()
            .map(|b| (b.duration_seconds * 10.0) as u64)
            .collect();
        let sparkline = Sparkline::default()
            .block(
                Block::default().borders(Borders::ALL).title("⏱️ Recent Build Times"),
            )
            .data(&sparkline_data)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(sparkline, chunks[2]);
    }
    fn render_performance(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        let recent: Vec<_> = self.data.builds.iter().rev().take(10).collect();
        let data: Vec<(f64, f64)> = recent
            .iter()
            .enumerate()
            .map(|(i, b)| (i as f64, b.duration_seconds))
            .collect();
        if !data.is_empty() {
            let datasets = vec![
                Dataset::default().name("Build Time").marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Cyan))
                .graph_type(ratatui::widgets::GraphType::Line).data(& data)
            ];
            let max_time = data.iter().map(|(_, t)| *t).fold(0.0, f64::max);
            let max_x = (data.len() - 1) as f64;
            let chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📈 Build Performance"),
                )
                .x_axis(
                    Axis::default()
                        .title("Recent Builds")
                        .style(Style::default().fg(Color::Gray))
                        .bounds([0.0, max_x]),
                )
                .y_axis(
                    Axis::default()
                        .title("Time (s)")
                        .style(Style::default().fg(Color::Gray))
                        .bounds([0.0, max_time * 1.2]),
                );
            frame.render_widget(chart, chunks[0]);
        } else {
            let placeholder = Paragraph::new(
                    "No build data yet. Run some cargo commands!",
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📈 Build Performance"),
                )
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(placeholder, chunks[0]);
        }
        let mut daily_summaries: Vec<_> = self.data.daily_summary.values().collect();
        daily_summaries.sort_by(|a, b| b.date.cmp(&a.date));
        if !daily_summaries.is_empty() {
            let formatted_dates: Vec<String> = daily_summaries
                .iter()
                .take(7)
                .map(|s| {
                    if s.date.starts_with("2025-") {
                        s.date[5..].to_string()
                    } else {
                        s.date.clone()
                    }
                })
                .collect();
            let daily_data: Vec<(&str, u64)> = formatted_dates
                .iter()
                .zip(daily_summaries.iter().take(7))
                .map(|(date_str, summary)| (
                    date_str.as_str(),
                    summary.total_builds as u64,
                ))
                .collect();
            let max_builds = daily_data.iter().map(|(_, v)| *v).max().unwrap_or(1);
            let bar_chart = BarChart::default()
                .block(Block::default().borders(Borders::ALL).title("📅 Daily Builds"))
                .data(&daily_data)
                .bar_width(6)
                .bar_gap(1)
                .max(max_builds)
                .style(Style::default().fg(Color::Yellow))
                .value_style(Style::default().fg(Color::Black).bg(Color::Yellow));
            frame.render_widget(bar_chart, chunks[1]);
        } else {
            let placeholder = Paragraph::new("No daily build data yet.")
                .block(Block::default().borders(Borders::ALL).title("📅 Daily Builds"))
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(placeholder, chunks[1]);
        }
    }
    fn render_dependencies(&self, frame: &mut Frame, area: Rect) {
        let mut deps: Vec<(&String, &DependencyMetrics)> = self
            .data
            .dependencies
            .iter()
            .collect();
        deps.sort_by(|a, b| {
            b.1.compile_time_seconds.partial_cmp(&a.1.compile_time_seconds).unwrap()
        });
        let items: Vec<ListItem> = deps
            .iter()
            .take(20)
            .map(|(name, metrics)| {
                let content = format!(
                    "{} v{} - {:.2}s", name, metrics.version, metrics
                    .compile_time_seconds
                );
                ListItem::new(content)
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📦 Dependency Compile Times"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");
        frame.render_widget(list, area);
    }
    fn render_trends(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        let trend_text = self.analyze_trends();
        let trends = Paragraph::new(trend_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📊 Build Trends Analysis"),
            )
            .style(Style::default().fg(Color::White))
            .wrap(ratatui::widgets::Wrap {
                trim: true,
            });
        frame.render_widget(trends, chunks[0]);
        let recommendations = self.get_recommendations();
        let rec_items: Vec<ListItem> = recommendations
            .iter()
            .map(|r| ListItem::new(r.as_str()))
            .collect();
        let rec_list = List::new(rec_items)
            .block(Block::default().borders(Borders::ALL).title("💡 Recommendations"))
            .style(Style::default().fg(Color::Green));
        frame.render_widget(rec_list, chunks[1]);
    }
    fn analyze_trends(&self) -> String {
        if self.data.builds.is_empty() {
            return "No build data available yet.".to_string();
        }
        let recent = &self.data.builds[self.data.builds.len().saturating_sub(100)..];
        let old = &self.data.builds[..self.data.builds.len().saturating_sub(100)];
        let recent_avg = if !recent.is_empty() {
            recent.iter().map(|b| b.duration_seconds).sum::<f64>() / recent.len() as f64
        } else {
            0.0
        };
        let old_avg = if !old.is_empty() {
            old.iter().map(|b| b.duration_seconds).sum::<f64>() / old.len() as f64
        } else {
            recent_avg
        };
        let improvement = ((old_avg - recent_avg) / old_avg * 100.0).abs();
        let trend = if recent_avg < old_avg {
            format!("✅ Build times improved by {:.1}%", improvement)
        } else if recent_avg > old_avg {
            format!("⚠️ Build times increased by {:.1}%", improvement)
        } else {
            "→ Build times stable".to_string()
        };
        let error_trend = self.analyze_error_trend();
        let peak_times = self.find_peak_build_times();
        format!("{}\n{}\n{}", trend, error_trend, peak_times)
    }
    fn analyze_error_trend(&self) -> String {
        let recent = &self.data.builds[self.data.builds.len().saturating_sub(50)..];
        let total_errors: usize = recent.iter().map(|b| b.error_count).sum();
        let total_warnings: usize = recent.iter().map(|b| b.warning_count).sum();
        format!("Recent: {} errors, {} warnings", total_errors, total_warnings)
    }
    fn find_peak_build_times(&self) -> String {
        if self.data.builds.is_empty() {
            return String::new();
        }
        let mut hour_builds: HashMap<u32, Vec<f64>> = HashMap::new();
        for build in &self.data.builds {
            let hour = build.timestamp.hour();
            hour_builds.entry(hour).or_default().push(build.duration_seconds);
        }
        let mut peak_hour = 0;
        let mut peak_avg = 0.0;
        for (hour, times) in &hour_builds {
            let avg = times.iter().sum::<f64>() / times.len() as f64;
            if avg > peak_avg {
                peak_avg = avg;
                peak_hour = *hour;
            }
        }
        format!("Peak build time: {}:00 (avg {:.2}s)", peak_hour, peak_avg)
    }
    fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        if self.get_avg_build_time() > 60.0 {
            recommendations
                .push("Consider using cargo-nextest for faster test runs".to_string());
            recommendations
                .push("Enable incremental compilation in Cargo.toml".to_string());
        }
        let recent_failures = self
            .data
            .builds
            .iter()
            .rev()
            .take(10)
            .filter(|b| !b.success)
            .count();
        if recent_failures > 3 {
            recommendations
                .push(
                    "High failure rate detected - consider running clippy".to_string(),
                );
            recommendations
                .push("Set up pre-commit hooks to catch issues earlier".to_string());
        }
        if self.data.dependencies.len() > 100 {
            recommendations
                .push("Large dependency count - audit with cargo-outdated".to_string());
            recommendations
                .push(
                    "Consider using cargo-machete to find unused dependencies"
                        .to_string(),
                );
        }
        let incremental_builds = self
            .data
            .builds
            .iter()
            .filter(|b| b.incremental)
            .count();
        if incremental_builds < self.data.builds.len() / 2 {
            recommendations
                .push("Enable incremental compilation for faster rebuilds".to_string());
        }
        recommendations
    }
    fn get_avg_build_time(&self) -> f64 {
        if self.data.builds.is_empty() {
            0.0
        } else {
            let total: f64 = self.data.builds.iter().map(|b| b.duration_seconds).sum();
            total / self.data.builds.len() as f64
        }
    }
    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.data_file, json)?;
        Ok(())
    }
    pub fn export_csv(&self, path: &PathBuf) -> Result<()> {
        let mut csv = String::new();
        csv.push_str("timestamp,command,duration,success,errors,warnings\n");
        for build in &self.data.builds {
            csv.push_str(
                &format!(
                    "{},{},{},{},{},{}\n", build.timestamp.to_rfc3339(), build.command,
                    build.duration_seconds, build.success, build.error_count, build
                    .warning_count
                ),
            );
        }
        fs::write(path, csv)?;
        println!("✅ Build metrics exported to {}", path.display());
        Ok(())
    }
}
impl Default for TideData {
    fn default() -> Self {
        Self {
            builds: Vec::new(),
            dependencies: HashMap::new(),
            daily_summary: HashMap::new(),
        }
    }
}
pub fn check_sailor_tracker(command: &str) -> Result<bool> {
    println!("⛵ Sailor tracking command '{}' - checking the winds!", command.cyan());
    let license_manager = license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Sailor reports: Command '{}' has fair winds!", command.green()
            );
            println!("   ⛵ All sails set - ready to catch the wind!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Sailor warning: Command wind exceeded!");
                println!("   ⛵ Change course to: https://cargo.do/checkout");
                println!("   ⛵ Upgrade for unlimited sailing commands");
            } else if e.to_string().contains("License not found") {
                println!("❌ Sailor emergency: No navigation charts found!");
                println!("   ⛵ Get charts with 'cm register <key>'");
            } else {
                println!(
                    "❌ Sailor distress: Command tracking failed: {}", e.to_string()
                    .red()
                );
                println!("   ⛵ Man overboard - secure the ship!");
            }
            Ok(false)
        }
    }
}