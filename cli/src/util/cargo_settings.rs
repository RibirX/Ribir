/// Forked and modified from Tauri CLI
/// https://github.com/tauri-apps/tauri/tree/dev/crates/tauri-cli
use std::{
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::Context;
use serde::Deserialize;
use serde_value::ValueDeserializer;

/// The Cargo settings (Cargo.toml root descriptor).
#[derive(Clone, Debug, Deserialize)]
pub struct CargoSettings {
  /// the package settings.
  ///
  /// it's optional because ancestor workspace Cargo.toml files may not have
  /// package info.
  pub package: Option<CargoPackageSettings>,
  /// the workspace settings.
  ///
  /// it's present if the read Cargo.toml belongs to a workspace root.
  pub workspace: Option<WorkspaceSettings>,
  /// the binary targets configuration.
  pub bin: Option<Vec<BinarySettings>>,
}

#[allow(unused)]
/// The `workspace` section of the app configuration (read from Cargo.toml).
#[derive(Clone, Debug, Deserialize)]
pub struct WorkspaceSettings {
  /// the workspace members.
  pub members: Option<Vec<String>>,
  pub package: Option<WorkspacePackageSettings>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TomlWorkspaceField {
  pub workspace: bool,
}

// Taken from https://github.com/rust-lang/cargo/blob/70898e522116f6c23971e2a554b2dc85fd4c84cd/src/cargo/util/toml/mod.rs#L1008-L1065
/// Enum that allows for the parsing of `field.workspace = true` in a Cargo.toml
///
/// It allows for things to be inherited from a workspace or defined as needed
#[derive(Clone, Debug)]
pub enum MaybeWorkspace<T> {
  Workspace(TomlWorkspaceField),
  Defined(T),
}

impl<'de, T: Deserialize<'de>> serde::de::Deserialize<'de> for MaybeWorkspace<T> {
  fn deserialize<D>(deserializer: D) -> Result<MaybeWorkspace<T>, D::Error>
  where
    D: serde::de::Deserializer<'de>,
  {
    let value = serde_value::Value::deserialize(deserializer)?;
    if let Ok(workspace) =
      TomlWorkspaceField::deserialize(ValueDeserializer::<D::Error>::new(value.clone()))
    {
      return Ok(MaybeWorkspace::Workspace(workspace));
    }
    T::deserialize(serde_value::ValueDeserializer::<D::Error>::new(value))
      .map(MaybeWorkspace::Defined)
  }
}

impl<T> MaybeWorkspace<T> {
  pub fn resolve(
    self, label: &str, get_ws_field: impl FnOnce() -> anyhow::Result<T>,
  ) -> anyhow::Result<T> {
    match self {
      MaybeWorkspace::Defined(value) => Ok(value),
      MaybeWorkspace::Workspace(TomlWorkspaceField { workspace: true }) => {
        get_ws_field().context(format!(
          "error inheriting `{label}` from workspace root manifest's `workspace.package.{label}`"
        ))
      }
      MaybeWorkspace::Workspace(TomlWorkspaceField { workspace: false }) => {
        Err(anyhow::anyhow!("`workspace=false` is unsupported for `package.{label}`"))
      }
    }
  }
  fn _as_defined(&self) -> Option<&T> {
    match self {
      MaybeWorkspace::Workspace(_) => None,
      MaybeWorkspace::Defined(defined) => Some(defined),
    }
  }
}

/// The package settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CargoPackageSettings {
  /// the package's name.
  pub name: String,
  /// the package's version.
  pub version: Option<MaybeWorkspace<String>>,
  /// the package's description.
  pub description: Option<MaybeWorkspace<String>>,
  /// the package's homepage.
  pub homepage: Option<MaybeWorkspace<String>>,
  /// the package's authors.
  pub authors: Option<MaybeWorkspace<Vec<String>>>,
  /// the package's license.
  pub license: Option<MaybeWorkspace<String>>,
  /// the default binary to run.
  pub default_run: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorkspacePackageSettings {
  pub authors: Option<Vec<String>>,
  pub description: Option<String>,
  pub homepage: Option<String>,
  pub version: Option<String>,
  pub license: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BinarySettings {
  pub name: String,
  /// This is from nightly: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#different-binary-name
  pub filename: Option<String>,
  pub path: Option<String>,
}

impl BinarySettings {
  /// The file name without the binary extension (e.g. `.exe`)
  pub fn file_name(&self) -> &str { self.filename.as_ref().unwrap_or(&self.name) }
}

pub fn read_toml<T: for<'b> Deserialize<'b>>(file_path: &PathBuf) -> crate::Result<T> {
  let toml_str = std::fs::read_to_string(file_path)
    .with_context(|| format!("Failed to read {}", file_path.display()))?;
  toml::from_str(&toml_str).with_context(|| format!("Failed to parse {}", file_path.display()))
}

impl CargoSettings {
  /// Try to load a set of CargoSettings from a "Cargo.toml" file in the
  /// specified directory.
  pub fn load(file_path: &PathBuf) -> crate::Result<Self> { read_toml(file_path) }

  pub fn load_from_dir(dir: &Path) -> crate::Result<Self> {
    let toml_path = dir.join("Cargo.toml");
    Self::load(&toml_path)
  }

  /// Search for a `Cargo.toml` file in the specified directory or its
  /// ancestors.
  pub fn toml_path(dir: &Path) -> Option<PathBuf> {
    let mut current_dir = dir;

    loop {
      let toml_path = current_dir.join("Cargo.toml");
      if toml_path.exists() {
        return Some(toml_path);
      }

      match current_dir.parent() {
        Some(parent) => current_dir = parent,
        None => break,
      }
    }

    None
  }
}

#[derive(Deserialize)]
pub(crate) struct CargoMetadata {
  pub(crate) target_directory: PathBuf,
  pub(crate) workspace_root: PathBuf,
}

pub fn get_cargo_metadata() -> crate::Result<CargoMetadata> {
  let output = Command::new("cargo")
    .args(["metadata", "--no-deps", "--format-version", "1"])
    .current_dir(std::env::current_dir()?)
    .output()?;

  if !output.status.success() {
    return Err(anyhow::anyhow!(
      "cargo metadata command exited with a non zero exit code: {}",
      String::from_utf8_lossy(&output.stderr)
    ));
  }

  Ok(serde_json::from_slice(&output.stdout)?)
}
pub fn get_workspace_dir() -> crate::Result<PathBuf> {
  Ok(
    get_cargo_metadata()
      .context("failed to get cargo metadata")?
      .workspace_root,
  )
}
