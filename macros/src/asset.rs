//! Asset processing framework for compile-time asset handling.
//!
//! This module provides the core infrastructure for the `asset!` and
//! `include_asset!` macros, enabling compile-time processing and runtime
//! loading of various asset types.
//!
//! # Architecture
//!
//! The framework separates concerns into two layers:
//!
//! - **Asset Trait**: Defines how specific asset types process data and
//!   generate load expressions. Implementors focus only on data transformation.
//! - **Framework Layer**: Handles all common logic including path resolution,
//!   file I/O, caching, embed/bundle decisions, and code generation.
//!
//! # Supported Asset Types
//!
//! | Type | Macro Syntax | Return Type | Description |
//! |------|--------------|-------------|-------------|
//! | Binary | `asset!("file.bin")` | `Vec<u8>` | Raw bytes, no processing |
//! | Text | `asset!("file.txt", "text")` | `String` | UTF-8 text |
//! | SVG | `asset!("file.svg", "svg")` | `Svg` | Compressed at compile-time |
//! | Image | `asset!("file.png", "image")` | `Image` | Converted to WebP |
//!
//! # Adding New Asset Types
//!
//! To add a new asset type, implement the [`Asset`] trait:
//!
//! ```ignore
//! struct MyAsset;
//!
//! impl Asset for MyAsset {
//!   fn process(&self, ctx: &AssetContext) -> syn::Result<Option<Vec<u8>>> {
//!     // Transform input data, or return None to use original
//!     let data = std::fs::read(&ctx.abs_input)?;
//!     Ok(Some(transform(data)))
//!   }
//!
//!   fn output_extension(&self) -> Option<&str> {
//!     Some("myext") // If output extension differs from input
//!   }
//!
//!   fn load_expr(&self, data_expr: TokenStream) -> TokenStream {
//!     quote! { MyType::from_bytes(#data_expr) }
//!   }
//! }
//! ```
//!
//! Then register it in [`AssetArgs::parse()`].

use std::{
  fs,
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
};

use ahash::AHasher;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitBool, LitStr, parse::Parse};

mod basic;
mod image;
mod svg;

use basic::{BinaryAsset, TextAsset};
use image::ImageAsset;
use svg::SvgAsset;

// =============================================================================
// Public API
// =============================================================================

/// Generates asset loading code for the `asset!` macro.
///
/// Assets are copied to the target directory and loaded at runtime. The
/// original file is tracked via `include_bytes!` to trigger recompilation on
/// changes.
///
/// # Path Resolution
///
/// Paths are resolved relative to the source file containing the macro call,
/// similar to `include_str!` in Rust.
///
/// # Examples
///
/// ```ignore
/// // Binary (default)
/// let data: Vec<u8> = asset!("image.png");
///
/// // Text
/// let config: String = asset!("config.json", "text");
///
/// // SVG with parameters
/// let icon: Svg = asset!("icon.svg", "svg", inherit_fill = true);
///
/// // Image (auto-converted to WebP)
/// let img: Image = asset!("photo.jpg", "image");
/// ```
pub fn gen_asset(input: TokenStream) -> TokenStream { gen_asset_internal(input, false) }

/// Generates asset embedding code for the `include_asset!` macro.
///
/// Unlike `asset!`, this embeds the processed asset directly into the binary,
/// similar to `include_bytes!`. No external files are needed at runtime.
///
/// # Examples
///
/// ```ignore
/// // Embed SVG directly in binary
/// let icon: Svg = include_asset!("icon.svg", "svg");
///
/// // Embed image (converted to WebP, then embedded)
/// let img: Image = include_asset!("photo.png", "image");
/// ```
pub fn gen_include_asset(input: TokenStream) -> TokenStream { gen_asset_internal(input, true) }

// =============================================================================
// Asset Trait
// =============================================================================

/// Trait for defining asset type handlers.
///
/// Implementors only need to focus on:
/// - **Data processing**: Transform input bytes (compression, format
///   conversion, etc.)
/// - **Output extension**: Specify if the output file extension differs from
///   input
/// - **Load expression**: Generate code to construct the final value from bytes
///
/// The framework handles all common concerns: path resolution, file I/O,
/// caching, embed vs runtime loading, and change tracking.
pub(crate) trait Asset {
  /// Process input data and return transformed bytes.
  ///
  /// Return `Some(data)` with processed bytes, or `None` to use the original
  /// input unchanged (e.g., for binary assets that need no transformation).
  fn process(&self, ctx: &AssetContext) -> syn::Result<Option<Vec<u8>>>;

  /// Return the output file extension if it differs from the input.
  ///
  /// For example, `ImageAsset` returns `Some("webp")` because it converts
  /// all image formats to WebP. Return `None` to keep the original extension.
  fn output_extension(&self) -> Option<&str> { None }

  /// Generate the load expression that constructs the final asset value.
  ///
  /// # Arguments
  ///
  /// * `data_expr` - A `Cow<[u8]>` expression containing either:
  ///   - Embedded bytes: `Cow::Borrowed(&[...])`
  ///   - Runtime read: `Cow::Owned(std::fs::read(...))`
  ///
  /// # Returns
  ///
  /// A `TokenStream` that evaluates to the final asset type (e.g., `Svg`,
  /// `Image`).
  fn load_expr(&self, data_expr: TokenStream) -> TokenStream;

  /// Return a hash of processing parameters for cache invalidation.
  ///
  /// When the same input file is processed with different parameters (e.g.,
  /// `inherit_fill = true` vs `false` for SVG), this hash ensures separate
  /// cached outputs.
  fn params_hash(&self) -> Option<String> { None }
}

// =============================================================================
// Code Generation
// =============================================================================

fn gen_asset_internal(input: TokenStream, embed: bool) -> TokenStream {
  match syn::parse2::<AssetArgs>(input).and_then(|args| process_and_generate(args, embed)) {
    Ok(ts) => ts,
    Err(e) => e.to_compile_error(),
  }
}

fn process_and_generate(args: AssetArgs, embed: bool) -> syn::Result<TokenStream> {
  let ctx = prepare_asset_context(&args.input, embed, args.asset.params_hash())?;
  generate_for_asset(args.asset.as_ref(), &ctx)
}

/// Unified code generation for all asset types.
///
/// This function handles the common logic:
/// 1. Calculate output path (with potential extension change)
/// 2. Process and cache the asset if outdated
/// 3. Generate either embedded bytes or runtime load code
/// 4. Wrap with `include_bytes!` for change tracking
fn generate_for_asset(asset: &dyn Asset, ctx: &AssetContext) -> syn::Result<TokenStream> {
  // Calculate output path (considering extension change)
  let output_path = match asset.output_extension() {
    Some(ext) => ctx.abs_output.with_extension(ext),
    None => ctx.abs_output.clone(),
  };

  // Process and write if needed
  let processed = if ctx.needs_update(&output_path) {
    let data = asset.process(ctx)?;
    match &data {
      Some(bytes) => {
        fs::write(&output_path, bytes).map_err(|e| ctx.error(format!("Write failed: {e}")))?;
      }
      None => {
        fs::copy(&ctx.abs_input, &output_path)
          .map_err(|e| ctx.error(format!("Copy failed: {e}")))?;
      }
    }
    data
  } else {
    None // Will read from cache if embedding
  };

  // Generate data expression (Cow<[u8]>)
  let data_expr = if ctx.embed {
    let bytes = match processed {
      Some(data) => data,
      None => fs::read(&output_path).map_err(|e| ctx.error(format!("Read cached: {e}")))?,
    };
    quote! { std::borrow::Cow::Borrowed(&[#(#bytes),*][..]) }
  } else {
    let path = ctx.runtime_path_tokens(asset.output_extension());
    quote! { std::borrow::Cow::<[u8]>::Owned(std::fs::read(#path).expect("Failed to read asset")) }
  };

  // Generate final expression with include_bytes for change tracking
  let abs_input = ctx.abs_input.to_string_lossy().into_owned();
  let load = asset.load_expr(data_expr);

  Ok(quote! {
    {
      // This ensures recompilation when the source file changes
      const _: &[u8] = include_bytes!(#abs_input);
      #load
    }
  })
}

// =============================================================================
// Macro Argument Parsing
// =============================================================================

/// Parsed arguments from `asset!` or `include_asset!` macro invocation.
pub struct AssetArgs {
  /// The input file path literal.
  pub input: LitStr,
  /// The asset handler for processing and code generation.
  asset: Box<dyn Asset>,
}

impl Parse for AssetArgs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let input_str = input.parse::<LitStr>()?;

    let asset: Box<dyn Asset> = if input.parse::<syn::Token![,]>().is_ok() {
      let type_str = input.parse::<LitStr>()?;
      match type_str.value().to_lowercase().as_str() {
        "text" => Box::new(TextAsset),
        "svg" => {
          let params = parse_key_value_params(input)?;
          let inherit_fill = get_bool_param(&params, "inherit_fill", type_str.span())?;
          let inherit_stroke = get_bool_param(&params, "inherit_stroke", type_str.span())?;
          Box::new(SvgAsset { inherit_fill, inherit_stroke })
        }
        "image" => Box::new(ImageAsset),
        _ => Box::new(BinaryAsset),
      }
    } else {
      Box::new(BinaryAsset)
    };

    Ok(AssetArgs { input: input_str, asset })
  }
}

/// Parsed parameter value (bool or string literal).
enum ParamValue {
  Bool(bool),
  String(String),
}

/// Parse optional key=value parameters after the type string.
fn parse_key_value_params(
  input: syn::parse::ParseStream,
) -> syn::Result<std::collections::HashMap<String, ParamValue>> {
  let mut params = std::collections::HashMap::new();

  while input.parse::<syn::Token![,]>().is_ok() {
    let key: Ident = input.parse()?;
    input.parse::<syn::Token![=]>()?;

    let value = if let Ok(b) = input.parse::<LitBool>() {
      ParamValue::Bool(b.value)
    } else if let Ok(s) = input.parse::<LitStr>() {
      ParamValue::String(s.value())
    } else {
      return Err(syn::Error::new(key.span(), "Expected bool or string literal"));
    };
    params.insert(key.to_string(), value);
  }
  Ok(params)
}

/// Extract a boolean parameter from the parsed params map.
fn get_bool_param(
  params: &std::collections::HashMap<String, ParamValue>, name: &str, span: proc_macro2::Span,
) -> syn::Result<bool> {
  params
    .get(name)
    .map(|v| match v {
      ParamValue::Bool(b) => Ok(*b),
      ParamValue::String(s) => {
        Err(syn::Error::new(span, format!("Expected bool for `{name}`, got string: `{s}`")))
      }
    })
    .transpose()
    .map(|opt| opt.unwrap_or(false))
}

// =============================================================================
// Asset Context
// =============================================================================

/// Context passed to asset handlers during processing.
///
/// Contains all path information and utilities needed by [`Asset`]
/// implementations.
pub(crate) struct AssetContext {
  /// Original input path from the macro (as written by user).
  pub input_path: String,
  /// Absolute path to the input file.
  pub abs_input: PathBuf,
  /// Absolute path to the output file in target directory.
  pub abs_output: PathBuf,
  /// Relative output path (for bundle mode).
  pub relative_output: String,
  /// Span for error reporting.
  pub input_span: proc_macro2::Span,
  /// Whether building in bundle mode (PROFILE=bundle).
  pub is_bundle: bool,
  /// Whether embedding (include_asset!) vs runtime loading (asset!).
  pub embed: bool,
}

impl AssetContext {
  /// Create a `syn::Error` with input path context.
  pub fn error(&self, msg: impl std::fmt::Display) -> syn::Error {
    syn::Error::new(self.input_span, format!("{} for '{}'", msg, self.input_path))
  }

  /// Check if processing is needed (source newer than output).
  pub fn needs_update(&self, output: &Path) -> bool {
    match (self.abs_input.metadata(), output.metadata()) {
      (Ok(i), Ok(o)) => match (i.modified(), o.modified()) {
        (Ok(it), Ok(ot)) => it > ot,
        _ => true,
      },
      _ => true,
    }
  }

  /// Generate runtime path expression for loading the asset.
  ///
  /// In bundle mode, generates path relative to executable.
  /// In debug mode, generates absolute path to target directory.
  pub fn runtime_path_tokens(&self, new_ext: Option<&str>) -> TokenStream {
    let relative = match new_ext {
      Some(ext) => {
        let stem = Path::new(&self.relative_output)
          .file_stem()
          .and_then(|s| s.to_str())
          .unwrap_or(&self.relative_output);
        let parent = Path::new(&self.relative_output)
          .parent()
          .and_then(|p| p.to_str())
          .unwrap_or("");
        if parent.is_empty() {
          format!("{}.{}", stem, ext)
        } else {
          format!("{}/{}.{}", parent, stem, ext)
        }
      }
      None => self.relative_output.clone(),
    };

    if self.is_bundle {
      quote! { std::env::current_exe().unwrap().parent().unwrap().join(#relative) }
    } else {
      let output = match new_ext {
        Some(ext) => self.abs_output.with_extension(ext),
        None => self.abs_output.clone(),
      };
      let abs = output.to_string_lossy().into_owned();
      quote! { std::path::PathBuf::from(#abs) }
    }
  }
}

// =============================================================================
// Path Resolution & Utilities
// =============================================================================

/// Prepare the asset context from macro input.
fn prepare_asset_context(
  input: &LitStr, embed: bool, params_hash: Option<String>,
) -> syn::Result<AssetContext> {
  let input_path = input.value();
  let span = input.span();
  let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
  let is_bundle = profile == "bundle";

  let abs_input = resolve_caller_relative_path(&input_path, span)?;

  if !abs_input.exists() {
    return Err(syn::Error::new(span, format!("Asset not found: {abs_input:?}")));
  }
  if !abs_input.is_file() {
    return Err(syn::Error::new(span, format!("Not a file: '{input_path}'")));
  }

  let filename = abs_input
    .file_name()
    .and_then(|n| n.to_str())
    .ok_or_else(|| syn::Error::new(span, "Invalid filename"))?;

  // Generate unique output filename using path hash + params hash
  // Use absolute path for hash to ensure same file = same cache regardless of how
  // it's referenced
  let abs_input_str = abs_input.to_string_lossy();
  let path_hash = hash_path(&abs_input_str);
  let hashed_filename = match params_hash {
    Some(ph) => format!("{path_hash}_{ph}_{filename}"),
    None => format!("{path_hash}_{filename}"),
  };

  let (manifest_path, workspace_opt) = get_workspace_base(span)?;
  let base_target = std::env::var_os("CARGO_TARGET_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      workspace_opt
        .clone()
        .unwrap_or_else(|| manifest_path.clone())
        .join("target")
    });

  let target_dir = base_target.join(&profile).join("assets");
  let abs_output = target_dir.join(&hashed_filename);

  if !target_dir.exists() {
    fs::create_dir_all(&target_dir)
      .map_err(|e| syn::Error::new(span, format!("Failed to create dir: {e}")))?;
  }

  let relative_output = format!("assets/{hashed_filename}");

  // Record in manifest for bundle packaging (skip for embedded assets)
  // Use absolute path in manifest for consistency
  if !embed {
    append_to_manifest(&abs_input_str, &relative_output, &target_dir, span)?;
  }

  Ok(AssetContext {
    input_path,
    abs_input,
    abs_output,
    relative_output,
    input_span: span,
    is_bundle,
    embed,
  })
}

fn resolve_caller_relative_path(input_path: &str, span: proc_macro2::Span) -> syn::Result<PathBuf> {
  let path = PathBuf::from(input_path);
  if path.is_absolute() {
    return Ok(path);
  }

  if input_path.starts_with("~/") || input_path == "~" {
    let home = std::env::var("HOME")
      .or_else(|_| std::env::var("USERPROFILE"))
      .map_err(|_| syn::Error::new(span, "Could not find home directory"))?;
    let tail = if input_path.len() > 1 { &input_path[2..] } else { "" };
    return Ok(PathBuf::from(home).join(tail));
  }

  let caller_file = span
    .unwrap()
    .local_file()
    .ok_or_else(|| syn::Error::new(span, "Cannot get source file path from span"))?;

  let caller_dir = caller_file
    .parent()
    .ok_or_else(|| syn::Error::new(span, format!("Invalid source path: {caller_file:?}")))?;

  let resolved = caller_dir.join(input_path);

  if resolved.is_absolute() {
    return Ok(resolved);
  }

  // When resolved is relative, we need to find the right base directory.
  // The span.local_file() behavior differs between contexts:
  // - In workspace: returns path relative to workspace root (e.g.,
  //   "core/src/file.rs")
  // - In cargo package: returns path relative to crate root (e.g., "src/file.rs")
  //
  // Strategy: Try CARGO_MANIFEST_DIR first, then workspace root as fallback.
  let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
    .map_err(|_| syn::Error::new(span, "CARGO_MANIFEST_DIR not set"))?;
  let manifest_path = PathBuf::from(&manifest_dir);

  // First try: resolve relative to CARGO_MANIFEST_DIR (works for cargo package)
  let candidate = manifest_path.join(&resolved);
  if candidate.exists() {
    return Ok(candidate);
  }

  // Second try: resolve relative to workspace root (works for workspace builds)
  if let Some(workspace_root) = find_workspace_root(&manifest_path) {
    let candidate = workspace_root.join(&resolved);
    if candidate.exists() {
      return Ok(candidate);
    }
  }

  // Return the manifest-based path for error reporting
  Ok(manifest_path.join(&resolved))
}

/// Get manifest directory and workspace root.
fn get_workspace_base(span: proc_macro2::Span) -> syn::Result<(PathBuf, Option<PathBuf>)> {
  let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
    .map_err(|_| syn::Error::new(span, "CARGO_MANIFEST_DIR not set"))?;
  let manifest_path = PathBuf::from(&manifest_dir);
  Ok((manifest_path.clone(), find_workspace_root(&manifest_path)))
}

/// Generate a short hash of the path for unique filenames.
fn hash_path(path: &str) -> String {
  let mut h = AHasher::default();
  path.hash(&mut h);
  format!("{:08x}", h.finish())
}

/// Append asset mapping to manifest file for bundle packaging.
fn append_to_manifest(
  input: &str, output: &str, target_dir: &Path, span: proc_macro2::Span,
) -> syn::Result<()> {
  use std::io::Write;
  let path = target_dir.join(".asset_manifest.txt");
  let mut file = fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .map_err(|e| syn::Error::new(span, format!("Manifest open failed: {e}")))?;
  writeln!(file, "{input} -> {output}")
    .map_err(|e| syn::Error::new(span, format!("Manifest write failed: {e}")))
}

/// Find workspace root by searching for Cargo.toml with [workspace].
fn find_workspace_root(start: &Path) -> Option<PathBuf> {
  let mut cur = Some(start);
  while let Some(p) = cur {
    let toml = p.join("Cargo.toml");
    if toml.exists() && fs::read_to_string(&toml).is_ok_and(|c| c.contains("[workspace]")) {
      return Some(p.to_path_buf());
    }
    cur = p.parent();
  }
  None
}
