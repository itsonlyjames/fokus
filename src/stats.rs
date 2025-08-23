use chrono::{Local, NaiveDate};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub daily_sessions: HashMap<String, u64>,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            daily_sessions: HashMap::new(),
        }
    }
}

impl SessionStats {
    fn get_stats_path() -> Result<PathBuf> {
        Ok(Config::get_config_dir()?.join("stats.toml"))
    }

    pub fn load_stats() -> Result<SessionStats> {
        let stats_path = Self::get_stats_path()?;
        if stats_path.exists() {
            let stats_str = fs::read_to_string(stats_path)?;
            let stats: SessionStats = toml::from_str(&stats_str)?;
            Ok(stats)
        } else {
            Ok(SessionStats::default())
        }
    }

    pub fn save_stats(stats: &SessionStats) -> Result<()> {
        let stats_path = Self::get_stats_path()?;
        let stats_str = toml::to_string_pretty(stats)?;
        fs::write(stats_path, stats_str)?;
        Ok(())
    }

    pub fn get_today_sessions(&self) -> u64 {
        let today = Local::now().date_naive().to_string();
        self.daily_sessions.get(&today).copied().unwrap_or(0)
    }

    pub fn increment_session(&mut self) {
        self.total_sessions += 1;

        let today = Local::now().date_naive().to_string();
        *self.daily_sessions.entry(today).or_insert(0) += 1;
    }

    pub fn get_sessions_for_date(&self, date: NaiveDate) -> u64 {
        let date_str = date.to_string();
        self.daily_sessions.get(&date_str).copied().unwrap_or(0)
    }

    pub fn get_total_sessions(&self) -> u64 {
        self.total_sessions
    }
}
