/// Forked and modified from Tauri CLI
/// https://github.com/tauri-apps/tauri/tree/dev/crates/tauri-cli
use std::{
  env,
  path::{Path, PathBuf},
  str::FromStr,
};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, FromArgMatches, Parser};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tauri_bundler::{
  AppCategory, AppImageSettings, BundleBinary, BundleSettings, DebianSettings, DmgSettings,
  Entitlements, IosSettings, MacOsSettings, PackageSettings, Position, RpmSettings,
  SettingsBuilder, Size, WindowsSettings, bundle_project,
};
use tauri_utils::config::{
  AndroidConfig, BundleResources, BundleTarget, CustomSignCommandConfig, FileAssociation,
  IosConfig, LinuxConfig, MacConfig, NsisConfig, Updater, WebviewInstallMode, WindowsConfig,
  WixConfig, WixLanguage,
};

use crate::{
  CliCommand,
  util::cargo_settings::{
    CargoPackageSettings, CargoSettings, get_cargo_metadata, get_workspace_dir, read_toml,
  },
};

pub fn bundle() -> Box<dyn CliCommand> { Box::new(BundleCmd {}) }

struct BundleCmd {}

#[derive(Parser, Debug, Clone)]
#[command(name = "bundle")]
/// run as web wasm
struct Bundle {
  /// Direction path to output, default to target/wasm
  #[arg(short, long)]
  config: Option<PathBuf>,

  /// verbose, default to false
  #[arg(short, long)]
  debug: bool,

  /// Direction path of the target dir
  #[arg(short, long)]
  target_dir: Option<PathBuf>,

  /// verbose, default to false
  #[arg(short, long)]
  verbose: bool,
}

pub fn wix_settings(config: WixConfig) -> tauri_bundler::WixSettings {
  tauri_bundler::WixSettings {
    version: config.version,
    upgrade_code: config.upgrade_code,
    language: tauri_bundler::WixLanguage(match config.language {
      WixLanguage::One(lang) => vec![(lang, Default::default())],
      WixLanguage::List(languages) => languages
        .into_iter()
        .map(|lang| (lang, Default::default()))
        .collect(),
      WixLanguage::Localized(languages) => languages
        .into_iter()
        .map(|(lang, config)| {
          (
            lang,
            tauri_bundler::WixLanguageConfig { locale_path: config.locale_path.map(Into::into) },
          )
        })
        .collect(),
    }),
    template: config.template,
    fragment_paths: config.fragment_paths,
    component_group_refs: config.component_group_refs,
    component_refs: config.component_refs,
    feature_group_refs: config.feature_group_refs,
    feature_refs: config.feature_refs,
    merge_refs: config.merge_refs,
    enable_elevated_update_task: config.enable_elevated_update_task,
    banner_path: config.banner_path,
    dialog_image_path: config.dialog_image_path,
    fips_compliant: true,
  }
}

pub fn nsis_settings(config: NsisConfig) -> tauri_bundler::NsisSettings {
  tauri_bundler::NsisSettings {
    template: config.template,
    header_image: config.header_image,
    sidebar_image: config.sidebar_image,
    installer_icon: config.installer_icon,
    install_mode: config.install_mode,
    languages: config.languages,
    custom_language_files: config.custom_language_files,
    display_language_selector: config.display_language_selector,
    compression: config.compression,
    start_menu_folder: config.start_menu_folder,
    installer_hooks: config.installer_hooks,
    minimum_webview2_version: config.minimum_webview2_version,
  }
}

pub fn custom_sign_settings(
  config: CustomSignCommandConfig,
) -> tauri_bundler::CustomSignCommandSettings {
  match config {
    CustomSignCommandConfig::Command(command) => {
      let mut tokens = command.split(' ');
      tauri_bundler::CustomSignCommandSettings {
        cmd: tokens.next().unwrap().to_string(), // split always has at least one element
        args: tokens.map(String::from).collect(),
      }
    }
    CustomSignCommandConfig::CommandWithOptions { cmd, args } => {
      tauri_bundler::CustomSignCommandSettings { cmd, args }
    }
  }
}

/// Cargo.toml shape we care about for bundling.
///
/// Supports both legacy `[bundle]` and the Cargo-recommended
/// `[package.metadata.bundle]` (which avoids `unused manifest key` warnings).
#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize, Debug, Default)]
struct CargoTomlBundle {
  bundle: Option<BundleConfig>,
  package: Option<CargoTomlPackage>,
}

#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize, Debug, Default)]
struct CargoTomlPackage {
  #[serde(default)]
  metadata: CargoTomlPackageMetadata,
}

#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize, Debug, Default)]
struct CargoTomlPackageMetadata {
  bundle: Option<BundleConfig>,
}

/// Configuration for bundle the app.
///
/// All relative paths in this configuration (such as `icon`, `resources`,
/// `externalBin`, `licenseFile`) are resolved relative to the config file's
/// directory, not the current working directory.
#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundleConfig {
  /// The application name.
  pub product_name: Option<String>,
  /// The application identifier in reverse domain name notation (e.g.
  /// `com.ribir.example`). This string must be unique across applications
  /// since it is used in system configurations like the bundle ID and path to
  /// the webview data directory. This string must contain only alphanumeric
  /// characters (A-Z, a-z, and 0-9), hyphens (-), and periods (.).
  pub identifier: Option<String>,

  /// The application version. if not set, will be read from Cargo.toml
  #[serde(default)]
  pub version: Option<String>,

  /// The bundle targets, currently supports ["deb", "rpm", "appimage", "nsis",
  /// "msi", "app", "dmg"] or "all".
  #[serde(default)]
  pub targets: BundleTarget,
  #[serde(default)]
  /// Produce updaters and their signatures or not
  pub create_updater_artifacts: Updater,
  /// The application's publisher. Defaults to the second element in the
  /// identifier string.
  ///
  /// Currently maps to the Manufacturer property of the Windows Installer
  /// and the Maintainer field of debian packages if the Cargo.toml does not
  /// have the authors field.
  pub publisher: Option<String>,
  /// A url to the home page of your application. If unset, will
  /// fallback to `homepage` defined in `Cargo.toml`.
  ///
  /// Supported bundle targets: `deb`, `rpm`, `nsis` and `msi`.
  pub homepage: Option<String>,
  /// The app's icons
  #[serde(default)]
  pub icon: Vec<String>,
  /// App resources to bundle.
  /// Each resource is a path to a file or directory.
  /// Glob patterns are supported.
  pub resources: Option<BundleResources>,
  /// A copyright string associated with your application.
  pub copyright: Option<String>,
  /// The package's license identifier to be included in the appropriate
  /// bundles. If not set, defaults to the license from the Cargo.toml file.
  pub license: Option<String>,
  /// The path to the license file to be included in the appropriate bundles.
  #[serde(alias = "license-file")]
  pub license_file: Option<PathBuf>,
  /// The application kind.
  ///
  /// Should be one of the following:
  /// Business, DeveloperTool, Education, Entertainment, Finance, Game,
  /// ActionGame, AdventureGame, ArcadeGame, BoardGame, CardGame, CasinoGame,
  /// DiceGame, EducationalGame, FamilyGame, KidsGame, MusicGame, PuzzleGame,
  /// RacingGame, RolePlayingGame, SimulationGame, SportsGame, StrategyGame,
  /// TriviaGame, WordGame, GraphicsAndDesign, HealthcareAndFitness, Lifestyle,
  /// Medical, Music, News, Photography, Productivity, Reference,
  /// SocialNetworking, Sports, Travel, Utility, Video, Weather.
  pub category: Option<String>,
  /// File associations to application.
  pub file_associations: Option<Vec<FileAssociation>>,
  /// A short description of your application.
  #[serde(alias = "short-description")]
  pub short_description: Option<String>,
  /// A longer, multi-line description of the application.
  #[serde(alias = "long-description")]
  pub long_description: Option<String>,
  /// Whether to use the project's `target` directory, for caching build tools
  /// (e.g., Wix and NSIS) when building this application. Defaults to `false`.
  ///
  /// If true, tools will be cached in `target/.tauri/`.
  /// If false, tools will be cached in the current user's platform-specific
  /// cache directory.
  ///
  /// An example where it can be appropriate to set this to `true` is when
  /// building this application as a Windows System user (e.g., AWS EC2
  /// workloads), because the Window system's app data directory is
  /// restricted.
  #[serde(default, alias = "use-local-tools-dir")]
  pub use_local_tools_dir: bool,
  /// A list of—either absolute or relative—paths to binaries to embed with your
  /// application.
  ///
  /// Note that Ribir will look for system-specific binaries following the
  /// pattern "binary-name{-target-triple}{.system-extension}".
  ///
  /// E.g. for the external binary "my-binary", Ribir looks for:
  ///
  /// - "my-binary-x86_64-pc-windows-msvc.exe" for Windows
  /// - "my-binary-x86_64-apple-darwin" for macOS
  /// - "my-binary-x86_64-unknown-linux-gnu" for Linux
  ///
  /// so don't forget to provide binaries for all targeted platforms.
  #[serde(alias = "external-bin")]
  pub external_bin: Option<Vec<String>>,
  /// Configuration for the Windows bundles.
  #[serde(default = "default_window_config")]
  pub windows: WindowsConfig,
  /// Configuration for the Linux bundles.
  #[serde(default)]
  pub linux: LinuxConfig,
  /// Configuration for the macOS bundles.
  #[serde(rename = "macOS", alias = "macos", default)]
  pub macos: MacConfig,
  /// iOS configuration.
  #[serde(rename = "iOS", alias = "ios", default)]
  pub ios: IosConfig,
  /// Android configuration.
  #[serde(default)]
  pub android: AndroidConfig,
}
fn default_window_config() -> WindowsConfig {
  WindowsConfig { webview_install_mode: WebviewInstallMode::Skip, ..Default::default() }
}

/// Resolve a relative path against a base directory.
/// Returns the path as-is if it's absolute.
fn resolve_path(path: &str, base_dir: &Path) -> String {
  let p = Path::new(path);
  if p.is_absolute() { path.to_string() } else { base_dir.join(p).to_string_lossy().into_owned() }
}

fn bundle_setting_from_config(
  config: BundleConfig, config_dir: &Path, package_settings: CargoPackageSettings,
) -> Result<BundleSettings> {
  let work_space_path = get_workspace_dir()?;
  let work_space = CargoSettings::load_from_dir(&work_space_path).ok();

  // Resolve resource paths relative to config directory
  let (resources, resources_map) = match config.resources {
    Some(BundleResources::List(paths)) => {
      let resolved: Vec<String> = paths
        .into_iter()
        .map(|p| resolve_path(&p, config_dir))
        .collect();
      (Some(resolved), None)
    }
    Some(BundleResources::Map(map)) => {
      let resolved = map
        .into_iter()
        .map(|(k, v)| (resolve_path(&k, config_dir), v))
        .collect();
      (None, Some(resolved))
    }
    None => (None, None),
  };

  let signing_identity = config.macos.signing_identity;

  let provider_short_name = config.macos.provider_short_name;
  #[allow(unused_mut)]
  let mut depends_deb = config.linux.deb.depends.unwrap_or_default();

  #[allow(unused_mut)]
  let mut depends_rpm = config.linux.rpm.depends.unwrap_or_default();

  #[allow(unused_mut)]
  let mut appimage_files = config.linux.appimage.files;
  Ok(BundleSettings {
    identifier: config.identifier,
    publisher: config.publisher,
    homepage: config.homepage,
    // Resolve icon paths relative to config directory
    icon: Some(
      config
        .icon
        .into_iter()
        .map(|p| resolve_path(&p, config_dir))
        .collect(),
    ),
    resources,
    resources_map,
    copyright: config.copyright,
    category: match config.category {
      Some(category) => Some(AppCategory::from_str(&category).map_err(|e| match e {
        Some(e) => anyhow::anyhow!("invalid category, did you mean `{}`?", e),
        None => anyhow::anyhow!("invalid category"),
      })?),
      None => None,
    },
    file_associations: config.file_associations,
    short_description: config.short_description,
    long_description: config.long_description,
    // Resolve external_bin paths relative to config directory
    external_bin: config.external_bin.map(|bins| {
      bins
        .into_iter()
        .map(|p| resolve_path(&p, config_dir))
        .collect()
    }),
    deb: DebianSettings {
      depends: if depends_deb.is_empty() { None } else { Some(depends_deb) },
      recommends: config.linux.deb.recommends,
      provides: config.linux.deb.provides,
      conflicts: config.linux.deb.conflicts,
      replaces: config.linux.deb.replaces,
      files: config.linux.deb.files,
      desktop_template: config.linux.deb.desktop_template,
      section: config.linux.deb.section,
      priority: config.linux.deb.priority,
      changelog: config.linux.deb.changelog,
      pre_install_script: config.linux.deb.pre_install_script,
      post_install_script: config.linux.deb.post_install_script,
      pre_remove_script: config.linux.deb.pre_remove_script,
      post_remove_script: config.linux.deb.post_remove_script,
    },
    appimage: AppImageSettings {
      files: appimage_files,
      bundle_media_framework: config.linux.appimage.bundle_media_framework,
      bundle_xdg_open: false,
    },
    rpm: RpmSettings {
      depends: if depends_rpm.is_empty() { None } else { Some(depends_rpm) },
      recommends: config.linux.rpm.recommends,
      provides: config.linux.rpm.provides,
      conflicts: config.linux.rpm.conflicts,
      obsoletes: config.linux.rpm.obsoletes,
      release: config.linux.rpm.release,
      epoch: config.linux.rpm.epoch,
      files: config.linux.rpm.files,
      desktop_template: config.linux.rpm.desktop_template,
      pre_install_script: config.linux.rpm.pre_install_script,
      post_install_script: config.linux.rpm.post_install_script,
      pre_remove_script: config.linux.rpm.pre_remove_script,
      post_remove_script: config.linux.rpm.post_remove_script,
      compression: config.linux.rpm.compression,
    },
    dmg: DmgSettings {
      background: config.macos.dmg.background,
      window_position: config
        .macos
        .dmg
        .window_position
        .map(|window_position| Position { x: window_position.x, y: window_position.y }),
      window_size: Size {
        width: config.macos.dmg.window_size.width,
        height: config.macos.dmg.window_size.height,
      },
      app_position: Position {
        x: config.macos.dmg.app_position.x,
        y: config.macos.dmg.app_position.y,
      },
      application_folder_position: Position {
        x: config.macos.dmg.application_folder_position.x,
        y: config.macos.dmg.application_folder_position.y,
      },
    },
    ios: IosSettings { bundle_version: config.ios.bundle_version },
    macos: MacOsSettings {
      frameworks: config.macos.frameworks,
      files: config.macos.files,
      bundle_version: config.macos.bundle_version,
      bundle_name: config.macos.bundle_name,
      minimum_system_version: config.macos.minimum_system_version,
      exception_domain: config.macos.exception_domain,
      signing_identity,
      hardened_runtime: config.macos.hardened_runtime,
      provider_short_name,
      entitlements: config
        .macos
        .entitlements
        .map(|p| Entitlements::Path(p.into())),
      info_plist: None,
      skip_stapling: false,
    },
    #[allow(deprecated)]
    windows: WindowsSettings {
      timestamp_url: config.windows.timestamp_url,
      tsp: config.windows.tsp,
      digest_algorithm: config.windows.digest_algorithm,
      certificate_thumbprint: config.windows.certificate_thumbprint,
      wix: config.windows.wix.map(wix_settings),
      nsis: config.windows.nsis.map(nsis_settings),
      icon_path: PathBuf::new(),
      webview_install_mode: config.windows.webview_install_mode,
      allow_downgrades: config.windows.allow_downgrades,
      sign_command: config
        .windows
        .sign_command
        .map(custom_sign_settings),
    },
    license: config.license.or_else(|| {
      package_settings.license.clone().map(|license| {
        license
          .resolve("license", || {
            work_space
              .as_ref()
              .and_then(|ws| ws.workspace.as_ref())
              .and_then(|w| w.package.as_ref())
              .and_then(|v| v.license.clone())
              .ok_or_else(|| anyhow::anyhow!("Couldn't inherit value for `license` from workspace"))
          })
          .unwrap()
      })
    }),
    // Resolve license_file path relative to config directory
    license_file: config
      .license_file
      .map(|l| if l.is_absolute() { l } else { config_dir.join(l) }),
    updater: None,
    ..Default::default()
  })
}

fn default_target_dir(is_debug: bool) -> Option<PathBuf> {
  get_cargo_metadata().ok().map(|mut v| {
    v.target_directory
      .push(if is_debug { "debug" } else { "release" });
    v.target_directory
  })
}

impl Bundle {
  fn bundle(&self) -> Result<()> {
    let package_path = CargoSettings::toml_path(&env::current_dir()?).expect("no cargo settings");
    let config_path: PathBuf = if let Some(target_dir) = &self.config {
      if target_dir.is_absolute() {
        target_dir.clone()
      } else {
        env::current_dir()
          .unwrap()
          .join(target_dir.clone())
      }
    } else {
      package_path.clone()
    };

    let CargoTomlBundle { bundle, package } = read_toml(&config_path)?;
    let bundle_config = bundle
      .or_else(|| package.and_then(|p| p.metadata.bundle))
      .ok_or_else(|| {
        anyhow::anyhow!(
          "no bundle config found in {} (expected `[bundle]` or `[package.metadata.bundle]`) ",
          config_path.display()
        )
      })?;

    let cargo_settings = CargoSettings::load(&package_path)?;
    let cargo_package_settings = cargo_settings
      .package
      .expect("no package settings");
    let package_setting =
      cargo_package_settings.resolve(&bundle_config.product_name, &bundle_config.version)?;

    let mut binaries = vec![];
    let mut expected_binary_names: Vec<String> = vec![];
    if let Some(bins) = &cargo_settings.bin {
      let default_run = cargo_package_settings
        .default_run
        .clone()
        .unwrap_or_default();
      for bin in bins {
        let file_name = bin.file_name();
        let is_main = file_name == cargo_package_settings.name || file_name == default_run;
        binaries.push(BundleBinary::with_path(file_name.to_owned(), is_main, bin.path.clone()));
        expected_binary_names.push(file_name.to_owned());
      }
    }
    if binaries.is_empty() {
      binaries.push(BundleBinary::new(cargo_package_settings.name.clone(), true));
      expected_binary_names.push(cargo_package_settings.name.clone());
    }

    // Preflight: tauri-bundler expects built binaries under
    // `target/{debug|release}`. We do NOT auto-build here (this CLI is intended
    // to be used as a released binary tool), so provide an actionable error
    // message instead.
    {
      let cargo_metadata = get_cargo_metadata().context("failed to get cargo metadata")?;
      let profile_dir = if self.debug { "debug" } else { "release" };
      let expected_dir = cargo_metadata.target_directory.join(profile_dir);
      let mut missing = vec![];
      for name in &expected_binary_names {
        let mut p = expected_dir.join(name);
        if cfg!(windows) {
          p.set_extension("exe");
        }
        if !p.exists() {
          missing.push(p);
        }
      }

      if !missing.is_empty() {
        let profile_flag = if self.debug { "" } else { "--release" };
        bail!(
          "Missing built binary(ies):\n  {}\n\nBuild them first, then re-run bundling. For \
           example:\n  cargo build --manifest-path {} {}\n\nNote: bundling copies binaries from \
           {}.",
          missing
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  "),
          package_path.display(),
          profile_flag,
          expected_dir.display(),
        );
      }
    }

    let package_types = bundle_config
      .targets
      .to_vec()
      .iter()
      .map(|t| t.clone().into())
      .collect();
    let bundle_setting = bundle_setting_from_config(
      bundle_config,
      config_path.parent().unwrap(),
      cargo_package_settings,
    )?;
    let settings = SettingsBuilder::new()
      .package_settings(package_setting)
      .bundle_settings(bundle_setting)
      .binaries(binaries)
      .project_out_directory(self.target_dir.clone().unwrap_or_else(|| {
        default_target_dir(self.debug).expect("Failed to get default target directory")
      }))
      .target(tauri_utils::platform::target_triple()?)
      .package_types(package_types)
      .log_level(log::Level::Info)
      .build()?;

    bundle_project(&settings)?;

    log::info!(
      "bundle success {:?}",
      settings
        .project_out_directory()
        .to_path_buf()
        .join("bundle")
    );

    Ok(())
  }
}

impl CliCommand for BundleCmd {
  fn name(&self) -> &str { "bundle" }

  fn command(&self) -> clap::Command { Bundle::command() }

  fn exec(&self, args: &clap::ArgMatches) -> Result<()> {
    let bundle = Bundle::from_arg_matches(args)?;
    bundle.bundle()?;
    Ok(())
  }
}

impl CargoPackageSettings {
  pub fn resolve(
    &self, product_name: &Option<String>, version: &Option<String>,
  ) -> anyhow::Result<PackageSettings> {
    let ws_package_settings = CargoSettings::load_from_dir(&get_workspace_dir()?)
      .context("failed to load cargo settings from workspace root")?
      .workspace
      .and_then(|v| v.package);
    Ok(PackageSettings {
      product_name: product_name
        .clone()
        .unwrap_or_else(|| self.name.clone()),
      version: version.clone().unwrap_or_else(|| {
        self
          .version
          .as_ref()
          .map(|v| {
            v.clone()
              .resolve("version", || {
                ws_package_settings
                  .as_ref()
                  .and_then(|p| p.version.clone())
                  .ok_or_else(|| {
                    anyhow::anyhow!("Couldn't inherit value for `version` from workspace")
                  })
              })
              .expect("failed to resolve version")
          })
          .unwrap()
      }),

      description: self
        .description
        .as_ref()
        .map(|v| {
          v.clone()
            .resolve("description", || {
              ws_package_settings
                .as_ref()
                .and_then(|p| p.description.clone())
                .ok_or_else(|| {
                  anyhow::anyhow!("Couldn't inherit value for `description` from workspace")
                })
            })
            .expect("failed to resolve description")
        })
        .unwrap(),
      homepage: self.homepage.as_ref().and_then(|v| {
        v.clone()
          .resolve("homepage", || {
            ws_package_settings
              .as_ref()
              .and_then(|p| p.homepage.clone())
              .ok_or_else(|| {
                anyhow::anyhow!("Couldn't inherit value for `homepage` from workspace")
              })
          })
          .ok()
      }),
      authors: self.authors.as_ref().and_then(|v| {
        v.clone()
          .resolve("authors", || {
            ws_package_settings
              .as_ref()
              .and_then(|p| p.authors.clone())
              .ok_or_else(|| anyhow::anyhow!("Couldn't inherit value for `authors` from workspace"))
          })
          .ok()
      }),
      default_run: self.default_run.clone(),
    })
  }
}
