use std::{
  fs,
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
};

use ahash::AHasher;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitBool, LitStr, parse::Parse};

/// Generate asset loading code - this is the main entry point
///
/// This function processes different types of assets with their specific
/// parameters.
///
/// # Path Resolution
///
/// Asset paths are resolved relative to the source file where the macro is
/// called, similar to how `#include` works in C/C++ or `include_str!` works in
/// Rust.
///
/// Example:
/// ```ignore
/// // In src/ui/components/button.rs
/// let icon: Svg = asset!("../icons/button.svg", "svg");
/// // Resolves to: src/ui/icons/button.svg
/// ```
///
/// # Supported Asset Types
///
/// ## Binary (default)
/// ```ignore
/// asset!("path/to/image.png")  // Returns: Vec<u8>
/// ```
/// Simply copies the file to the assets directory and generates code to read it
/// as bytes.
///
/// ## Text
/// ```ignore
/// asset!("path/to/config.json", "text")  // Returns: String
/// asset!("path/to/config.json", "TEXT")  // Case-insensitive
/// ```
/// Copies the file and generates code to read it as UTF-8 text.
///
/// ## SVG
/// ```ignore
/// asset!("path/to/icon.svg", "svg")  // Returns: Svg
/// asset!("path/to/icon.svg", "SVG", inherit_fill = true)  // With parameters
/// asset!("path/to/icon.svg", "svg", inherit_fill = true, inherit_stroke = false)
/// ```
/// Compresses the SVG at compile time and generates code to deserialize it at
/// runtime. Supports optional parameters (key=value format):
/// - `inherit_fill`: Whether to inherit fill styles from parent (default:
///   false)
/// - `inherit_stroke`: Whether to inherit stroke styles from parent (default:
///   false)
///
/// # Complete Example
///
/// ```ignore
/// // Binary assets (default type)
/// let image_data: Vec<u8> = asset!("images/logo.png");
/// let font_data: Vec<u8> = asset!("fonts/roboto.ttf");
///
/// // Text assets (case-insensitive)
/// let config: String = asset!("config.json", "text");
/// let shader: String = asset!("shaders/vertex.glsl", "TEXT");
///
/// // SVG assets with compile-time compression
/// let icon: Svg = asset!("icons/menu.svg", "svg");
/// let styled_icon: Svg = asset!("icons/button.svg", "SVG", inherit_fill = true);
/// let fully_styled: Svg = asset!("icons/app.svg", "svg", inherit_fill = true, inherit_stroke = false);
/// ```
///
/// # Adding New Asset Types
///
/// To add a new asset type:
/// 1. Add a new variant to `AssetType` enum with its parameters
/// 2. Update `AssetArgs::parse()` to handle the new type string and parse its
///    parameters
/// 3. Add a new `process_xxx_asset()` function following the DRY pattern
/// 4. Add a match arm in `asset_impl()` to call your processor
pub fn gen_asset(input: TokenStream) -> TokenStream {
  match syn::parse2::<AssetArgs>(input).and_then(asset_impl) {
    Ok(ts) => ts,
    Err(e) => e.to_compile_error(),
  }
}

pub enum AssetType {
  Binary,
  Text,
  Svg { inherit_fill: bool, inherit_stroke: bool },
}

pub struct AssetArgs {
  pub input: LitStr,
  pub asset_type: AssetType,
}

impl Parse for AssetArgs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let input_str = input.parse::<LitStr>()?;
    let asset_type = if input.parse::<syn::Token![,]>().is_ok() {
      let type_str = input.parse::<LitStr>()?;
      let type_name = type_str.value().to_lowercase();

      match type_name.as_str() {
        "text" => AssetType::Text,
        "svg" => {
          let params = parse_key_value_params(input)?;
          let inherit_fill = get_bool_param(&params, "inherit_fill", type_str.span())?;
          let inherit_stroke = get_bool_param(&params, "inherit_stroke", type_str.span())?;
          AssetType::Svg { inherit_fill, inherit_stroke }
        }
        _ => AssetType::Binary,
      }
    } else {
      AssetType::Binary
    };

    Ok(AssetArgs { input: input_str, asset_type })
  }
}

fn get_bool_param(
  params: &std::collections::HashMap<String, ParamValue>, name: &str, span: proc_macro2::Span,
) -> syn::Result<bool> {
  params
    .get(name)
    .map(|v| match v {
      ParamValue::Bool(b) => Ok(*b),
      ParamValue::String(s) => Err(syn::Error::new(
        span,
        format!("Expected boolean for `{}`, found string: `{}`", name, s),
      )),
    })
    .transpose()
    .map(|opt| opt.unwrap_or(false))
}

fn parse_key_value_params(
  input: syn::parse::ParseStream,
) -> syn::Result<std::collections::HashMap<String, ParamValue>> {
  let mut params = std::collections::HashMap::new();

  while input.parse::<syn::Token![,]>().is_ok() {
    let key: Ident = input.parse()?;
    input.parse::<syn::Token![=]>()?;

    if let Ok(bool_val) = input.parse::<LitBool>() {
      params.insert(key.to_string(), ParamValue::Bool(bool_val.value));
    } else if let Ok(str_val) = input.parse::<LitStr>() {
      params.insert(key.to_string(), ParamValue::String(str_val.value()));
    } else {
      return Err(syn::Error::new(
        key.span(),
        "Parameter value must be a boolean or string literal",
      ));
    }
  }

  Ok(params)
}

enum ParamValue {
  Bool(bool),
  String(String),
}

fn hash_path(path: &str) -> String {
  let mut hasher = AHasher::default();
  path.hash(&mut hasher);
  format!("{:08x}", hasher.finish())
}

struct AssetContext {
  input_path: String,
  abs_input: PathBuf,
  abs_output: PathBuf,
  relative_output: String,
  input_span: proc_macro2::Span,
  is_bundle: bool,
}

fn asset_impl(args: AssetArgs) -> syn::Result<TokenStream> {
  let ctx = prepare_asset_context(&args)?;

  println!("cargo:rerun-if-changed={}", ctx.abs_input.display());

  match args.asset_type {
    AssetType::Binary => process_simple_asset(&ctx, "binary", quote! { std::fs::read }),
    AssetType::Text => process_simple_asset(&ctx, "text", quote! { std::fs::read_to_string }),
    AssetType::Svg { inherit_fill, inherit_stroke } => {
      process_svg_asset(&ctx, inherit_fill, inherit_stroke)
    }
  }
}

fn prepare_asset_context(args: &AssetArgs) -> syn::Result<AssetContext> {
  let input_path = args.input.value();
  let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
  let is_bundle = profile == "bundle";

  let abs_input = resolve_caller_relative_path(&input_path, args.input.span())?;

  if !abs_input.exists() {
    let err_msg = format!("Asset file '{}' does not exist at: {:?}", input_path, abs_input);
    return Err(syn::Error::new_spanned(&args.input, err_msg));
  }

  if !abs_input.is_file() {
    let err_msg =
      format!("Asset path '{}' is not a file. Only single files are supported.", input_path);
    return Err(syn::Error::new_spanned(&args.input, err_msg));
  }

  let filename = abs_input
    .file_name()
    .and_then(|n| n.to_str())
    .ok_or_else(|| syn::Error::new_spanned(&args.input, "Failed to extract filename"))?;

  // Use path hash to avoid conflicts between files with same name from different
  // directories
  let path_hash = hash_path(&input_path);
  let hashed_filename = format!("{}_{}", path_hash, filename);

  let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| {
    syn::Error::new(args.input.span(), format!("`CARGO_MANIFEST_DIR` not set: {e}"))
  })?;
  let manifest_path = PathBuf::from(&manifest_dir);

  let workspace_opt = find_workspace_root(&manifest_path);

  let base_target_dir = std::env::var_os("CARGO_TARGET_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      workspace_opt
        .clone()
        .unwrap_or_else(|| manifest_path.clone())
        .join("target")
    });

  let target_dir = base_target_dir.join(&profile).join("assets");
  let abs_output = target_dir.join(&hashed_filename);

  fs::create_dir_all(&target_dir).map_err(|e| {
    let err_msg = format!("Failed to create asset output directory: {}", e);
    syn::Error::new(args.input.span(), err_msg)
  })?;

  let relative_output = format!("assets/{}", hashed_filename);

  // Append to manifest file for tracking asset mappings
  append_to_manifest(&input_path, &relative_output, &target_dir, args.input.span())?;

  Ok(AssetContext {
    input_path,
    abs_input,
    abs_output,
    relative_output,
    input_span: args.input.span(),
    is_bundle,
  })
}

fn copy_asset_file(ctx: &AssetContext, asset_type_name: &str) -> syn::Result<()> {
  fs::copy(&ctx.abs_input, &ctx.abs_output).map_err(|e| {
    let err_msg = format!("Failed to copy {} asset '{}': {}", asset_type_name, ctx.input_path, e);
    syn::Error::new(ctx.input_span, err_msg)
  })?;
  Ok(())
}

fn generate_asset_path_code(ctx: &AssetContext) -> TokenStream {
  if ctx.is_bundle {
    let relative_path = &ctx.relative_output;
    quote! {
      {
        let exe_dir = std::env::current_exe()
          .expect("Failed to get executable path")
          .parent()
          .expect("Failed to get executable directory")
          .to_path_buf();
        exe_dir.join(#relative_path)
      }
    }
  } else {
    let abs_output = ctx.abs_output.to_string_lossy();
    quote! {
      std::path::Path::new(#abs_output).to_path_buf()
    }
  }
}

fn process_simple_asset(
  ctx: &AssetContext, asset_type: &str, read_fn: TokenStream,
) -> syn::Result<TokenStream> {
  copy_asset_file(ctx, asset_type)?;

  let relative_path = &ctx.relative_output;
  let asset_path_code = generate_asset_path_code(ctx);

  Ok(quote! {
    {
      let asset_path = #asset_path_code;
      #read_fn(&asset_path)
        .unwrap_or_else(|e| panic!(
          "Failed to read {} asset '{}': {}. Asset path: {:?}",
          #asset_type, #relative_path, e, asset_path
        ))
    }
  })
}

fn process_svg_asset(
  ctx: &AssetContext, inherit_fill: bool, inherit_stroke: bool,
) -> syn::Result<TokenStream> {
  let compressed_data = ribir_painter::Svg::open(&ctx.abs_input, inherit_fill, inherit_stroke)
    .and_then(|svg| svg.serialize())
    .map_err(|e| {
      let err_msg = format!("Failed to compress SVG file '{}': {}", ctx.input_path, e);
      syn::Error::new(ctx.input_span, err_msg)
    })?;

  fs::write(&ctx.abs_output, &compressed_data).map_err(|e| {
    let err_msg = format!("Failed to write compressed SVG '{}': {}", ctx.input_path, e);
    syn::Error::new(ctx.input_span, err_msg)
  })?;

  let relative_path = &ctx.relative_output;
  let asset_path_code = generate_asset_path_code(ctx);

  Ok(quote! {
    {
      let asset_path = #asset_path_code;
      let data = std::fs::read_to_string(&asset_path)
        .unwrap_or_else(|e| panic!(
          "Failed to read SVG asset '{}': {}. Asset path: {:?}",
          #relative_path, e, asset_path
        ));
      Svg::deserialize(&data)
        .unwrap_or_else(|e| panic!(
          "Failed to deserialize SVG asset '{}': {}",
          #relative_path, e
        ))
    }
  })
}

fn resolve_caller_relative_path(input_path: &str, span: proc_macro2::Span) -> syn::Result<PathBuf> {
  let proc_macro_span = span.unwrap();
  let caller_file_path = proc_macro_span.local_file().ok_or_else(|| {
    syn::Error::new(
      span,
      "Failed to get source file path from span. This may happen with synthetic spans or when the \
       source file doesn't exist on disk.",
    )
  })?;

  let caller_dir = caller_file_path.parent().ok_or_else(|| {
    syn::Error::new(
      span,
      format!("Failed to get directory from caller's source file path: {:?}", caller_file_path),
    )
  })?;

  let abs_input = if Path::new(input_path).is_absolute() {
    PathBuf::from(input_path)
  } else {
    caller_dir.join(input_path)
  };

  Ok(abs_input)
}

fn append_to_manifest(
  input_path: &str, output_path: &str, target_dir: &Path, span: proc_macro2::Span,
) -> syn::Result<()> {
  let manifest_path = target_dir.join(".asset_manifest.txt");
  let entry = format!("{} -> {}\n", input_path, output_path);

  fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&manifest_path)
    .and_then(|mut file| {
      use std::io::Write;
      file.write_all(entry.as_bytes())
    })
    .map_err(|e| {
      let err_msg = format!("Failed to append to asset manifest file: {}", e);
      syn::Error::new(span, err_msg)
    })?;

  Ok(())
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
  let mut current = Some(start);

  while let Some(cur) = current {
    let cargo_toml = cur.join("Cargo.toml");
    if cargo_toml.exists()
      && let Ok(content) = fs::read_to_string(&cargo_toml)
      && content.contains("[workspace]")
    {
      return Some(cur.to_path_buf());
    }
    current = cur.parent();
  }

  None
}
