//! Port registry for debug server discovery.
//!
//! When the debug server starts, it registers its port in a well-known
//! location so that MCP clients can discover it automatically based on
//! the current working directory.

use std::{
  collections::hash_map::DefaultHasher,
  fs,
  hash::{Hash, Hasher},
  io::{self, Write},
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

/// Port registration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortEntry {
  pub port: u16,
  pub project_path: PathBuf,
  pub pid: u32,
  pub started_at: u64,
}

/// Get the state directory for port registrations.
///
/// - Linux/macOS: `~/.local/state/ribir/debug-ports/`
/// - Windows: `%LOCALAPPDATA%\ribir\debug-ports\`
fn get_state_dir() -> PathBuf {
  let base = dirs::state_dir()
    .or_else(dirs::data_local_dir)
    .unwrap_or_else(|| PathBuf::from("/tmp"));

  base.join("ribir").join("debug-ports")
}

/// Compute a hash of the path for use as a filename.
fn path_to_hash(path: &Path) -> String {
  let canonical = path
    .canonicalize()
    .unwrap_or_else(|_| path.to_path_buf());
  let path_str = canonical.to_string_lossy();

  let mut hasher = DefaultHasher::new();
  path_str.hash(&mut hasher);
  format!("{:016x}", hasher.finish())
}

/// Register the debug server port for the current working directory.
pub fn register_port(port: u16) -> io::Result<PathBuf> {
  let project_path = std::env::current_dir()?;
  let state_dir = get_state_dir();
  fs::create_dir_all(&state_dir)?;

  let hash = path_to_hash(&project_path);
  let file_path = state_dir.join(format!("{}.json", hash));

  let entry = PortEntry {
    port,
    project_path: project_path.clone(),
    pid: std::process::id(),
    started_at: SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_secs(),
  };

  let json = serde_json::to_string_pretty(&entry)
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

  let mut file = fs::File::create(&file_path)?;
  file.write_all(json.as_bytes())?;

  log::info!("Registered debug port {} for project: {}", port, project_path.display());

  Ok(file_path)
}

/// Unregister the debug server port.
pub fn unregister_port(registry_file: &Path) {
  if let Err(e) = fs::remove_file(registry_file) {
    if e.kind() != io::ErrorKind::NotFound {
      log::warn!("Failed to unregister debug port: {}", e);
    }
  } else {
    log::info!("Unregistered debug port");
  }
}
