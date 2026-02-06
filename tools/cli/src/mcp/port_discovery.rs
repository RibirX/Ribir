//! Port discovery for MCP debug server.
//!
//! Enables automatic discovery of the debug server port for the current
//! working directory, allowing multiple Ribir projects to be debugged
//! simultaneously without port conflicts.

use std::{
  fs,
  io::Read,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// Port registration entry stored in the state directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortEntry {
  pub port: u16,
  pub project_path: PathBuf,
  pub pid: u32,
  pub started_at: u64,
}

/// Registry for managing debug port registrations.
pub struct PortRegistry {
  state_dir: PathBuf,
}

impl PortRegistry {
  /// Create a new registry using the default state directory.
  pub fn new() -> Self { Self { state_dir: get_state_dir() } }

  /// Discover the port for a given project path.
  pub fn discover_for_path(&self, path: &Path) -> Option<PortEntry> {
    let hash = path_to_hash(path);
    let file_path = self.state_dir.join(format!("{}.json", hash));

    if !file_path.exists() {
      return None;
    }

    let entry = self.read_entry(&file_path)?;

    // Verify the process is still alive
    if !is_process_alive(entry.pid) {
      // Clean up stale entry
      let _ = fs::remove_file(&file_path);
      return None;
    }

    Some(entry)
  }

  /// List all registered ports.
  pub fn list_all(&self) -> Vec<PortEntry> {
    let mut entries = Vec::new();

    let Ok(dir) = fs::read_dir(&self.state_dir) else {
      return entries;
    };

    for entry in dir.flatten() {
      let path = entry.path();
      if path.extension().is_some_and(|ext| ext == "json") {
        if let Some(port_entry) = self.read_entry(&path) {
          if is_process_alive(port_entry.pid) {
            entries.push(port_entry);
          } else {
            // Clean up stale entry
            let _ = fs::remove_file(&path);
          }
        }
      }
    }

    entries
  }

  fn read_entry(&self, path: &Path) -> Option<PortEntry> {
    let mut file = fs::File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    serde_json::from_str(&contents).ok()
  }
}

impl Default for PortRegistry {
  fn default() -> Self { Self::new() }
}

/// Get the state directory for port registrations.
///
/// - Linux/macOS: `~/.local/state/ribir/debug-ports/`
/// - Windows: `%LOCALAPPDATA%\ribir\debug-ports\`
pub fn get_state_dir() -> PathBuf {
  let base = dirs::state_dir()
    .or_else(dirs::data_local_dir)
    .unwrap_or_else(|| PathBuf::from("/tmp"));

  base.join("ribir").join("debug-ports")
}

/// Compute a hash of the path for use as a filename.
/// Uses a simple hash to avoid filesystem issues with long paths.
pub fn path_to_hash(path: &Path) -> String {
  use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
  };

  let canonical = path
    .canonicalize()
    .unwrap_or_else(|_| path.to_path_buf());
  let path_str = canonical.to_string_lossy();

  let mut hasher = DefaultHasher::new();
  path_str.hash(&mut hasher);
  format!("{:016x}", hasher.finish())
}

/// Check if a process with the given PID is alive.
fn is_process_alive(pid: u32) -> bool {
  #[cfg(unix)]
  {
    // On Unix, sending signal 0 checks if process exists
    unsafe { libc::kill(pid as i32, 0) == 0 }
  }

  #[cfg(windows)]
  {
    use windows_sys::Win32::{
      Foundation::{CloseHandle, HANDLE},
      System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    unsafe {
      let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
      if handle == 0 {
        false
      } else {
        CloseHandle(handle);
        true
      }
    }
  }

  #[cfg(not(any(unix, windows)))]
  {
    // Fallback: assume alive
    true
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_path_to_hash() {
    let path1 = Path::new("project1");
    let path2 = Path::new("project2");

    let hash1 = path_to_hash(path1);
    let hash2 = path_to_hash(path2);

    assert_ne!(hash1, hash2);
    assert_eq!(hash1.len(), 16);
    assert_eq!(hash2.len(), 16);
  }

  #[test]
  fn test_same_path_same_hash() {
    let path = Path::new("project");
    let hash1 = path_to_hash(path);
    let hash2 = path_to_hash(path);

    assert_eq!(hash1, hash2);
  }
}
