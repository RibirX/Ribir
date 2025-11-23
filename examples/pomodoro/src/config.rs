use std::{fs, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone)]
pub struct PomodoroConfig {
  #[serde(with = "duration_serde")]
  pub focus: Duration,
  #[serde(with = "duration_serde")]
  pub short_break: Duration,
  #[serde(with = "duration_serde")]
  pub long_break: Duration,
  pub cycles: u32,
  #[serde(default)]
  pub always_on_top: bool,
  #[serde(default)]
  pub auto_run: bool,
  #[serde(default)]
  pub start_mini_mode: bool,
}

mod duration_serde {
  use std::time::Duration;

  use serde::{Deserialize, Deserializer, Serialize, Serializer};

  pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    duration.as_secs().serialize(serializer)
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
  where
    D: Deserializer<'de>,
  {
    let secs = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
  }
}

impl PomodoroConfig {
  pub fn default_config() -> Self {
    PomodoroConfig {
      focus: Duration::from_secs(1500),      // 25 minutes
      short_break: Duration::from_secs(300), // 5 minutes
      long_break: Duration::from_secs(900),  // 15 minutes
      cycles: 4,
      always_on_top: false,
      auto_run: false,
      start_mini_mode: true,
    }
  }

  fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ribir_pomodoro");
    if !path.exists() {
      let _ = fs::create_dir_all(&path);
    }
    path.push("config.json");
    path
  }

  pub fn load() -> Self {
    let path = Self::get_config_path();
    if let Ok(contents) = fs::read_to_string(&path)
      && let Ok(config) = serde_json::from_str::<PomodoroConfig>(&contents)
    {
      println!("Loaded config from: {:?}", path);
      return config;
    }
    println!("Using default config");
    Self::default_config()
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn save(&self) {
    let path = Self::get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(self) {
      if let Err(e) = fs::write(&path, json) {
        eprintln!("Failed to save config: {}", e);
      } else {
        println!("Config saved to: {:?}", path);
      }
    }
  }
}
