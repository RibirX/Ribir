#![allow(clippy::needless_lifetimes)]

extern crate proc_macro;

mod declare_derive;
mod dollar_macro;
mod lerp_derive;
mod part_state;
mod util;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};
mod asset;
mod child_template;
mod fn_widget_macro;
mod pipe_macro;
mod rdl_macro;
mod simple_declare_attr;
mod watch_macro;
pub(crate) use rdl_macro::*;
pub(crate) mod declare_obj;
pub(crate) mod distinct_pipe_macro;
pub(crate) mod error;
pub(crate) mod symbol_process;

#[proc_macro_derive(SingleChild)]
pub fn single_child_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;

  quote! {
    impl #impl_generics SingleChild for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(MultiChild)]
pub fn multi_child_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
    impl #impl_generics MultiChild for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(Lerp)]
pub fn lerp_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  lerp_derive::lerp_derive(&mut input)
    .unwrap_or_else(|e| e.into_compile_error())
    .into()
}

/// Macro to implement the `Declare` trait and build a `FatObj<T>`.
/// To know how to use it see the [`declare` mod document](declare)
///
/// This macro implement a `XXXBuilder` struct with same field type for `XXX`
/// widget, then
///
/// - implement `Declare` for `XXX`  mark `XXXBuilder` as its builder type.
/// - implement `ObjDeclarer` for `XXXBuilder` which build `XXX` and used by
///   `declare!` to build the `XXX` widget.
/// - for every field of `XXXBuilder`
///   - implement method with same name of the field and use to init the field.
///
/// [declare]: ../ribir/declare/index.html
#[proc_macro_derive(Declare, attributes(declare))]
pub fn declare_trait_macro_derive(input: TokenStream) -> TokenStream {
  let mut stt = parse_macro_input!(input as syn::ItemStruct);
  declare_derive::declare_derive(&mut stt)
    .unwrap_or_else(|e| e.into_compile_error())
    .into()
}

/// Macro attribute implement the `Declare` trait with that only build a
/// `State<T>` that not extend any built-in ability, and not support `pipe!` to
/// init the field.
#[proc_macro_attribute]
pub fn simple_declare(attr: TokenStream, item: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(item as syn::ItemStruct);
  let stateless = syn::parse::<syn::Ident>(attr).is_ok_and(|i| i == "stateless");
  simple_declare_attr::simple_declarer_attr(&mut input, stateless)
    .unwrap_or_else(|e| e.into_compile_error())
    .into()
}

#[proc_macro_derive(Template, attributes(template))]
pub fn child_template_trait_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  child_template::derive_child_template(&mut input)
    .unwrap_or_else(|e| e.into_compile_error())
    .into()
}

#[allow(clippy::doc_lazy_continuation)]
/// A macro use to declare an object. This macro will use `ctx!()` to access
/// the `BuildCtx`, so it can only use in a scope that has a `BuildCtx` named as
/// `ctx!()`.
///
/// # The Syntax
///
/// `rdl` accept 3 kind of syntax:
///
/// - 1. use struct literal syntax to declare a object tree, like `rdl!{ Row {
///   wrap: true } }`, if the `Row` contain any child, its child can be embed in
///   the struct literal, but must be use `rdl!` or `@` to declare, like:
///
///   ```ignore
///     rdl!{ Row { wrap: true, rdl!{ Text { text: "hello" } } } }
///   ```
/// - 2. similar to the first, but use a variable as parent and not accept any
///   fields of the parent(the builtin fields allowed), like:
///
///   ```ignore
///     let row = rdl!{ Row { wrap: true } };
///     rdl!{ $row { rdl!{ Text { text: "hello" } } } }
///   ```
/// - 3. use expression to declare a object and not allow declare children,
///   like: `let row = rdl!{ Widget::new(Void) };`
#[proc_macro]
pub fn rdl(input: TokenStream) -> TokenStream { RdlMacro::gen_code(input.into(), None).into() }

/// The `fn_widget` macro generates a widget from a function widget based on an
/// expression.
///
/// Its syntax extends the Rust syntax, allowing the use of `@` and `$` within
/// the expression. The `@` serves as a shorthand for the `rdl` macro, while
/// `$name` is used to express a state reference to `name`.
#[proc_macro]
pub fn fn_widget(input: TokenStream) -> TokenStream {
  fn_widget_macro::gen_code(input.into(), None).into()
}

/// This macro just return the input token stream. It's do nothing but help
/// `ribir` mark that a macro has been expanded.
#[proc_macro]
pub fn ribir_expanded_ಠ_ಠ(input: TokenStream) -> TokenStream { input }

/// This macro is utilized for generating a `Pipe` object that actively monitors
/// the expression's result.
///
/// The `$` symbol denotes the state reference and automatically subscribes to
/// any changes made to it. It triggers when the `$` state modifies.
#[proc_macro]
pub fn pipe(input: TokenStream) -> TokenStream { pipe_macro::gen_code(input.into(), None).into() }

/// A shorthand macro for `pipe!` can be utilized as follows:
/// `pipe!(...).value_chain(|s| s.distinct_until_changed().box_it())`.
///
/// It triggers when the new result differs from the previous one. The `$`
/// symbol denotes the state reference and automatically subscribes to any
/// changes made to it.
#[proc_macro]
pub fn distinct_pipe(input: TokenStream) -> TokenStream {
  distinct_pipe_macro::gen_code(input.into(), None).into()
}

/// The `watch!` macro converts an expression into an `Observable` stream. Use
/// `$` to mark the state reference, which automatically maps its modifications
/// to the expression value.
///
/// ## Example
///
/// ```rust ignore
/// use ribir::prelude::*;
///
/// let label = Stateful::new(1);
/// watch!(*$label)
///   .subscribe(|v| println!("{v}") );
///
/// *label.write() = 2;
/// ```
/// After subscribing, the subscription remains active until the state is fully
/// dropped.
///
/// ## Notice
///
/// If you use the `writer` of the watched state downstream, it introduces a
/// circular reference, preventing the state from being dropped. You need to
/// manually call unsubscribe at the appropriate time, typically in the
/// `on_disposed` method of a widget.
///
/// ```rust ignore
/// use ribir::prelude::*;
///
/// let even = Stateful::new(1);
/// let u = watch!(*$even).subscribe(move |v| {
///   if v % 2 == 1 {
///     *even.write() = (v + 1);
///   }
/// });
///
/// // ...
///
/// // Call unsubscribe at the appropriate time to ensure the state can be dropped.
/// u.unsubscribe();
/// ```
#[proc_macro]
pub fn watch(input: TokenStream) -> TokenStream { watch_macro::gen_code(input.into(), None).into() }

/// The `part_writer` macro creates a partial writer from a mutable reference of
/// a writer.
///
/// This macro specifically accepts simple expressions to indicate the partial
/// of the writer, as shown in the following patterns:
///
/// - For a field: `part_writer!(&mut writer.xxx)`
/// - For a method returning a mutable reference: `part_writer!(writer.xxx())`.
///
/// Since it operates on a writer and not a state reference of the writer, the
/// use of `$` is unnecessary.
#[proc_macro]
pub fn part_writer(input: TokenStream) -> TokenStream {
  part_state::gen_part_writer(input.into(), None).into()
}

/// The `part_watcher` macro creates a partial watcher from a reference of a
/// watcher.
///
/// This macro specifically accepts simple expressions to indicate the partial
/// of the watcher, as shown in the following patterns:
///
/// - For a field: `part_watcher!(&watcher.xxx)`
/// - For a method returning a reference: `part_watcher!(watcher.xxx())`.
///
/// Since it operates on a watcher and not a state reference of the watcher, the
/// use of `$` is unnecessary.
#[proc_macro]
pub fn part_watcher(input: TokenStream) -> TokenStream {
  part_state::gen_part_watcher(input.into(), None).into()
}

/// The `part_reader` macro creates a partial reader from a reference of a
/// reader.
///
/// This macro specifically accepts simple expressions to indicate the partial
/// of the reader, as shown in the following patterns:
///
/// - For a field: `part_reader!(&reader.xxx)`
/// - For a method returning a reference: `part_reader!(reader.xxx())`.
///
/// Since it operates on a reader and not a state reference of the reader, the
/// use of `$` is unnecessary.
#[proc_macro]
pub fn part_reader(input: TokenStream) -> TokenStream {
  part_state::gen_part_reader(input.into(), None).into()
}

/// Loads an asset file at runtime after processing it during compilation.
///
/// This macro manages application assets by:
/// 1. Processing the asset at compile time (format conversion, compression,
///    etc.)
/// 2. Copying the processed file to `target/{profile}/assets/`
/// 3. Generating code to load it from the filesystem at runtime
/// 4. Automatically triggering rebuilds when the source file changes
///
/// # Supported Types
///
/// | Type | Syntax | Return | Processing |
/// |------|--------|--------|------------|
/// | Binary | `asset!("file.bin")` | `Vec<u8>` | None (raw copy) |
/// | Text | `asset!("file.txt", "text")` | `String` | None (raw copy) |
/// | SVG | `asset!("file.svg", "svg")` | `Svg` | Serialized/compressed |
/// | Image | `asset!("file.png", "image")` | `Image` | Converted to WebP |
///
/// # Syntax
///
/// ```ignore
/// // Binary (default)
/// asset!("path/to/file.bin")
///
/// // Text
/// asset!("path/to/file.txt", "text")
///
/// // SVG with optional parameters
/// asset!("path/to/icon.svg", "svg")
/// asset!("path/to/icon.svg", "svg", inherit_fill = true)
/// asset!("path/to/icon.svg", "svg", inherit_fill = true, inherit_stroke = true)
///
/// // Image (PNG, JPEG, GIF, BMP → WebP)
/// asset!("path/to/photo.png", "image")
/// asset!("path/to/animation.gif", "image")  // Animated GIF → Animated WebP
/// ```
///
/// # Path Resolution
///
/// Paths are resolved **relative to the source file** where the macro is
/// called, similar to `include_str!` in Rust.
///
/// ```ignore
/// // In src/ui/widgets/button.rs
/// let icon: Svg = asset!("../icons/button.svg", "svg");
/// // Resolves to: src/ui/icons/button.svg
/// ```
///
/// # Type Parameters
///
/// ## SVG Parameters
/// - `inherit_fill`: Inherit fill style from parent widget (default: false)
/// - `inherit_stroke`: Inherit stroke style from parent widget (default: false)
///
/// # asset! vs include_asset!
///
/// | Feature | `asset!` | `include_asset!` |
/// |---------|----------|------------------|
/// | Loading | Runtime (filesystem) | Compile-time (embedded) |
/// | Distribution | Bundle `assets/` folder | Single binary |
/// | Binary Size | Smaller | Larger |
/// | Hot Reload | Possible | Requires recompile |
///
/// # Examples
///
/// ```ignore
/// // Binary assets
/// let font: Vec<u8> = asset!("fonts/roboto.ttf");
/// let data: Vec<u8> = asset!("data/model.bin");
///
/// // Text assets
/// let config: String = asset!("config/settings.json", "text");
/// let shader: String = asset!("shaders/fragment.glsl", "text");
///
/// // SVG assets (compressed at compile-time)
/// let icon: Svg = asset!("icons/menu.svg", "svg");
/// let themed: Svg = asset!("icons/logo.svg", "svg", inherit_fill = true);
///
/// // Image assets (converted to WebP)
/// let photo: Image = asset!("images/hero.jpg", "image");
/// let anim: Image = asset!("images/loading.gif", "image");  // Animated!
/// ```
///
/// # Bundling for Distribution
///
/// Include the `assets/` folder alongside your executable:
/// - **macOS**: `YourApp.app/Contents/MacOS/assets/`
/// - **Windows/Linux**: Same directory as executable
///
/// # Panics
///
/// Runtime panic if:
/// - Asset file not found at expected location
/// - File read error (permissions, I/O)
/// - UTF-8 decode error (text mode only)
///
/// # Compile Errors
///
/// Compile-time error if:
/// - Source file does not exist
/// - Path is a directory
/// - Cannot create output directory
/// - Cannot process/copy file
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream { asset::gen_asset(input.into()).into() }

/// Embeds an asset directly into the executable binary.
///
/// Similar to `asset!`, but embeds the processed file content directly into
/// the binary instead of loading from filesystem at runtime. This results in
/// a larger binary but simpler distribution (single file, no external assets).
///
/// # When to Use
///
/// - Single-binary distribution is preferred
/// - Assets are small and few
/// - No need for hot-reloading
///
/// # Syntax
///
/// Same as `asset!`:
///
/// ```ignore
/// include_asset!("file.bin")                        // → Vec<u8>
/// include_asset!("file.txt", "text")                // → String
/// include_asset!("icon.svg", "svg")                 // → Svg
/// include_asset!("photo.png", "image")              // → Image (WebP)
/// include_asset!("anim.gif", "image")               // → Image (animated WebP)
/// ```
///
/// # Returns
///
/// Same types as `asset!`:
/// - `Vec<u8>` for binary
/// - `String` for text
/// - `Svg` for SVG
/// - `Image` for image
#[proc_macro]
pub fn include_asset(input: TokenStream) -> TokenStream {
  asset::gen_include_asset(input.into()).into()
}
