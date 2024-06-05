#![cfg_attr(feature = "nightly", feature(proc_macro_span))]

extern crate proc_macro;

mod declare_derive;
mod lerp_derive;
mod util;
use proc_macro::TokenStream;
use quote::quote;
use symbol_process::DollarRefsCtx;
use syn::{parse_macro_input, DeriveInput};
mod child_template;
mod fn_widget_macro;
mod pipe_macro;
mod rdl_macro;
mod simple_declare_attr;
pub(crate) mod variable_names;
mod watch_macro;
pub(crate) use rdl_macro::*;
pub(crate) mod declare_obj;
pub(crate) mod error;
pub(crate) mod symbol_process;

macro_rules! ok {
  ($e:expr) => {
    match $e {
      Ok(ok) => ok,
      Err(err) => return err.into(),
    }
  };
}
pub(crate) use ok;

#[proc_macro_derive(SingleChild)]
pub fn single_marco_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
    impl #impl_generics SingleChild for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(Query)]
pub fn query_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
    impl #impl_generics Query for #name #ty_generics #where_clause {
      #[inline]
      fn query_inside_first(
        &self,
        type_id: TypeId,
        callback: &mut dyn FnMut(&dyn Any) -> bool
      )-> bool {
        self.query_outside_first(type_id, callback)
      }

      #[inline]
      fn query_outside_first(
        &self,
        type_id: TypeId,
        callback: &mut dyn FnMut(&dyn Any) -> bool
      ) -> bool{
        if type_id == self.type_id() {
          callback(self)
        } else {
          true
        }
      }
    }
  }
  .into()
}

#[proc_macro_derive(MultiChild)]
pub fn multi_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
    impl #impl_generics MultiChild for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(PairChild)]
pub fn pair_compose_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
    impl #impl_generics PairChild for #name #ty_generics #where_clause {}
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
///  [declare]: ../ribir/declare/index.html
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
  let mut refs = DollarRefsCtx::top_level();
  refs.new_dollar_scope(false);
  RdlMacro::gen_code(input.into(), &mut refs)
}

/// The `fn_widget` is a macro that create a widget from a function widget from
/// a expression. Its syntax is extended from rust syntax, you can use `@` and
/// `$` in the expression, the `@` is a short hand of `rdl` macro, and `$name`
/// use to expression a state reference of `name`.
#[proc_macro]
pub fn fn_widget(input: TokenStream) -> TokenStream {
  fn_widget_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level())
}

/// This macro just return the input token stream. It's do nothing but help
/// `ribir` mark that a macro has been expanded.
#[proc_macro]
pub fn ribir_expanded_ಠ_ಠ(input: TokenStream) -> TokenStream { input }

/// The `ctx!` macro is a special name that use to share the `BuildCtx` between
/// macros.
#[proc_macro]
pub fn ctx(input: TokenStream) -> TokenStream {
  let tokens = if !input.is_empty() {
    quote!(compile_error!("ctx! macro does not accept any argument"))
  } else {
    quote! { _ctx_ಠ_ಠ }
  };
  tokens.into()
}

/// `pipe` macro use to create `Pipe` object that continuous trace the
/// expression modify. Use the `$` mark the state reference and auto subscribe
/// to its modify.
#[proc_macro]
pub fn pipe(input: TokenStream) -> TokenStream {
  pipe_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level())
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
  watch_macro::gen_code(input.into(), &mut DollarRefsCtx::top_level())
}

/// Includes an SVG file as an `Svg`.

/// The file is located relative to the current crate (similar to the location
/// of your `cargo.toml`). The provided path is interpreted in a
/// platform-specific way at compile time. For example, a Windows path with
/// backslashes \ would not compile correctly on Unix.

/// This macro returns an expression of type `Svg`.
#[proc_macro]
pub fn include_crate_svg(input: TokenStream) -> TokenStream {
  let file = parse_macro_input! { input as syn::LitStr };
  let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  let path = std::path::Path::new(&dir).join(file.value());
  include_svg_from_path(path)
}

/// Includes an SVG file as an `Svg`.

/// The file is located relative to the current file (similarly to how modules
/// are found). The provided path is interpreted in a platform-specific way at
/// compile time. For example, a Windows path with backslashes \ would not
/// compile correctly on Unix.

/// This macro returns an expression of type `Svg`.
#[cfg(feature = "nightly")]
#[proc_macro]
pub fn include_svg(input: TokenStream) -> TokenStream {
  let rf = parse_macro_input! { input as syn::LitStr };

  let mut span = proc_macro::Span::call_site();
  while let Some(p) = span.parent() {
    span = p;
  }
  let mut file = span.source_file().path();
  file.pop();
  file.push(rf.value());

  include_svg_from_path(file)
}

fn include_svg_from_path(path: std::path::PathBuf) -> TokenStream {
  let encoded_bytes =
    ribir_painter::Svg::open(path.as_path()).and_then(|reader| reader.serialize());
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
