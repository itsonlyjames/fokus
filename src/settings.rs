use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub working_time: u64,
    pub break_time: u64,
    pub long_break_time: u64,
    pub sessions_until_long_break: u64,
}

impl Settings {
    pub fn default() -> Self {
        Self {
            working_time: 25,
            break_time: 5,
            long_break_time: 15,
            sessions_until_long_break: 2,
        }
    }

    pub fn get_working_time_seconds(&self) -> u64 {
        self.working_time * 60
    }

    pub fn get_break_time_seconds(&self) -> u64 {
        self.break_time * 60
    }

    pub fn get_long_break_time_seconds(&self) -> u64 {
        self.long_break_time * 60
    }
}

#[derive(Debug)]
pub enum Screen {
    Timer,
    Settings,
}

#[derive(Debug)]
pub enum SettingsField {
    WorkingTime,
    BreakTime,
    LongBreakTime,
    SessionsUntilLongBreak,
}
