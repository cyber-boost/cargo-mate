use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub command: String,
    pub success: bool,
    pub warnings: usize,
    pub errors: usize,
    pub duration_ms: u64,
}
#[derive(Debug, Clone)]
pub struct BuildStats {
    pub total_builds: usize,
    pub success_rate: f64,
    pub avg_warnings: f64,
    pub avg_errors: f64,
    pub last_success: Option<BuildRecord>,
    pub last_failure: Option<BuildRecord>,
    pub trend: BuildTrend,
}
#[derive(Debug, Clone)]
pub enum BuildTrend {
    Improving,
    Stable,
    Degrading,
}
pub struct BuildTracker {
    history: VecDeque<BuildRecord>,
    history_file: PathBuf,
    max_history: usize,
}
impl BuildTracker {
    pub fn new() -> Result<Self> {
        let history_file = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".shipwreck")
            .join("build_history.json");
        if let Some(parent) = history_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let history = if history_file.exists() {
            let content = fs::read_to_string(&history_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            VecDeque::new()
        };
        Ok(Self {
            history,
            history_file,
            max_history: 1000,
        })
    }
    pub fn add_record(&mut self, record: BuildRecord) -> Result<()> {
        self.history.push_front(record);
        while self.history.len() > self.max_history {
            self.history.pop_back();
        }
        self.save()?;
        Ok(())
    }
    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.history)?;
        fs::write(&self.history_file, json)?;
        Ok(())
    }
    pub fn get_stats(&self) -> BuildStats {
        let recent: Vec<&BuildRecord> = self.history.iter().take(50).collect();
        BuildStats {
            total_builds: self.history.len(),
            success_rate: self.calculate_success_rate(&recent),
            avg_warnings: self.calculate_avg_warnings(&recent),
            avg_errors: self.calculate_avg_errors(&recent),
            last_success: self.last_successful_build(),
            last_failure: self.last_failed_build(),
            trend: self.calculate_trend(),
        }
    }
    fn calculate_success_rate(&self, recent: &[&BuildRecord]) -> f64 {
        if recent.is_empty() {
            return 0.0;
        }
        let successes = recent.iter().filter(|r| r.success).count();
        successes as f64 / recent.len() as f64
    }
    fn calculate_avg_warnings(&self, recent: &[&BuildRecord]) -> f64 {
        if recent.is_empty() {
            return 0.0;
        }
        let total: usize = recent.iter().map(|r| r.warnings).sum();
        total as f64 / recent.len() as f64
    }
    fn calculate_avg_errors(&self, recent: &[&BuildRecord]) -> f64 {
        if recent.is_empty() {
            return 0.0;
        }
        let total: usize = recent.iter().map(|r| r.errors).sum();
        total as f64 / recent.len() as f64
    }
    fn last_successful_build(&self) -> Option<BuildRecord> {
        self.history.iter().find(|r| r.success).cloned()
    }
    fn last_failed_build(&self) -> Option<BuildRecord> {
        self.history.iter().find(|r| !r.success).cloned()
    }
    fn calculate_trend(&self) -> BuildTrend {
        if self.history.len() < 10 {
            return BuildTrend::Stable;
        }
        let recent: Vec<&BuildRecord> = self.history.iter().take(10).collect();
        let previous: Vec<&BuildRecord> = self
            .history
            .iter()
            .skip(10)
            .take(10)
            .collect();
        let recent_issues = recent.iter().map(|r| r.errors + r.warnings).sum::<usize>();
        let previous_issues = previous
            .iter()
            .map(|r| r.errors + r.warnings)
            .sum::<usize>();
        if recent_issues < previous_issues {
            BuildTrend::Improving
        } else if recent_issues > previous_issues {
            BuildTrend::Degrading
        } else {
            BuildTrend::Stable
        }
    }
    pub fn get_builds_for_command(&self, command: &str) -> Vec<BuildRecord> {
        self.history.iter().filter(|r| r.command.contains(command)).cloned().collect()
    }
    pub fn get_builds_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<BuildRecord> {
        self.history
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .cloned()
            .collect()
    }
}