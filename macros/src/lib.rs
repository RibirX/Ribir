#![allow(clippy::needless_lifetimes)]
#![cfg_attr(feature = "nightly", feature(proc_macro_span))]

extern crate proc_macro;

mod declare_derive;
mod lerp_derive;
mod part_writer;
mod util;
use proc_macro::TokenStream;
use quote::quote;
use symbol_process::DollarRefsCtx;
use syn::{DeriveInput, parse_macro_input};
mod child_template;
mod fn_widget_macro;
mod pipe_macro;
mod rdl_macro;
mod simple_declare_attr;
pub(crate) mod variable_names;
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
  let mut input = parse_macro_input!(input as DeriveInput);
  declare_derive::declare_derive(&mut input)
    .unwrap_or_else(|e| e.into_compile_error())
    .into()
}

/// Macro attribute implement the `Declare` trait with that only build a
/// `State<T>` that not extend any built-in ability, and not support `pipe!` to
/// init the field.
#[proc_macro_attribute]
pub fn simple_declare(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(item as syn::ItemStruct);
  simple_declare_attr::simple_declarer_attr(&mut input)
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
pub fn rdl(input: TokenStream) -> TokenStream {
  RdlMacro::gen_code(input.into(), &mut DollarRefsCtx::top_level()).into()
}

/// The `fn_widget` macro generates a widget from a function widget based on an
/// expression.
///
/// Its syntax extends the Rust syntax, allowing the use of `@` and `$` within
/// the expression. The `@` serves as a shorthand for the `rdl` macro, while
/// `$name` is used to express a state reference to `name`.
#[proc_macro]
pub fn fn_widget(input: TokenStream) -> TokenStream {
  fn_widget_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level()).into()
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
pub fn pipe(input: TokenStream) -> TokenStream {
  pipe_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level()).into()
}

/// Macro used to define a class to override for a `ClassName`, this is a
/// shorthand if you only want to compose builtin widgets with your host widget.
#[proc_macro]
pub fn style_class(input: TokenStream) -> TokenStream {
  let input: proc_macro2::TokenStream = input.into();
  quote! {
    (move |widget: Widget| {
      fn_widget! {
        let widget = FatObj::new(widget);
        @ $widget { #input }
      }.into_widget()
    }) as fn(Widget) -> Widget
  }
  .into()
}

/// A shorthand macro for `pipe!` can be utilized as follows:
/// `pipe!(...).value_chain(|s| s.distinct_until_changed().box_it())`.
///
/// It triggers when the new result differs from the previous one. The `$`
/// symbol denotes the state reference and automatically subscribes to any
/// changes made to it.
#[proc_macro]
pub fn distinct_pipe(input: TokenStream) -> TokenStream {
  distinct_pipe_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level()).into()
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
pub fn watch(input: TokenStream) -> TokenStream {
  watch_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level()).into()
}

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
  part_writer::gen_code(input.into(), &mut DollarRefsCtx::top_level()).into()
}

/// Includes an SVG file as an `Svg`.
///
/// The file is located relative to the current crate (similar to the location
/// of your `cargo.toml`). The provided path is interpreted in a
/// platform-specific way at compile time. For example, a Windows path with
/// backslashes \ would not compile correctly on Unix.
///
/// This macro returns an expression of type `Svg`.
#[proc_macro]
pub fn include_crate_svg(input: TokenStream) -> TokenStream {
  let IncludeSvgArgs { path, inherit_fill, inherit_stroke } =
    parse_macro_input! { input as IncludeSvgArgs };
  let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  let path = std::path::Path::new(&dir).join(path);
  include_svg_from_path(path, inherit_fill, inherit_stroke)
}

/// Includes an SVG file as an `Svg`.
///
/// The file is located relative to the current file (similarly to how modules
/// are found). The provided path is interpreted in a platform-specific way at
/// compile time. For example, a Windows path with backslashes \ would not
/// compile correctly on Unix.
///
/// This macro returns an expression of type `Svg`.
#[cfg(feature = "nightly")]
#[proc_macro]
pub fn include_svg(input: TokenStream) -> TokenStream {
  let IncludeSvgArgs { path, inherit_fill, inherit_stroke } =
    parse_macro_input! { input as IncludeSvgArgs };

  let mut span = proc_macro::Span::call_site();
  while let Some(p) = span.parent() {
    span = p;
  }
  let mut file = span.source_file().path();
  file.pop();
  file.push(path);

  include_svg_from_path(file, inherit_fill, inherit_stroke)
}

fn include_svg_from_path(
  path: std::path::PathBuf, inherit_fill: bool, inherit_stroke: bool,
) -> TokenStream {
  let encoded_bytes = ribir_painter::Svg::open(path.as_path(), inherit_fill, inherit_stroke)
    .and_then(|reader| reader.serialize());
  match encoded_bytes {
    Ok(data) => quote! {
      Svg::deserialize(#data).unwrap()
    }
    .into(),
    Err(err) => {
      let err = format!("{err}({:?})", &path);
      quote! { compile_error!(#err) }.into()
    }
  }
}

struct IncludeSvgArgs {
  path: String,
  inherit_fill: bool,
  inherit_stroke: bool,
}

impl syn::parse::Parse for IncludeSvgArgs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let path = input.parse::<syn::LitStr>()?.value();
    input.parse::<syn::Token![,]>()?;
    let inherit_fill = input.parse::<syn::LitBool>()?.value;
    input.parse::<syn::Token![,]>()?;
    let inherit_stroke = input.parse::<syn::LitBool>()?.value;

    Ok(IncludeSvgArgs { path, inherit_fill, inherit_stroke })
  }
}
