#![feature(proc_macro_diagnostic, proc_macro_span)]
extern crate proc_macro;

mod declare_derive;
mod lerp_derive;
mod util;
use fn_widget_macro::FnWidgetMacro;
use proc_macro::TokenStream;
use quote::quote;
use symbol_process::DollarRefsCtx;
use syn::{parse_macro_input, DeriveInput};
mod child_template;
mod fn_widget_macro;
mod pipe_macro;
mod rdl_macro;
pub(crate) mod variable_names;
mod watch_macro;
mod writer_map_macro;
pub(crate) use rdl_macro::*;

use crate::pipe_macro::PipeMacro;
use crate::watch_macro::WatchMacro;
pub(crate) mod declare_obj;
pub(crate) mod symbol_process;

macro_rules! ok {
  ($e: expr) => {
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

/// Macro to implement the `Declare` trait. To know how to use it see the
/// [`declare` mod document](declare)
///
/// This macro implement a `XXXBuilder` struct with same field type for `XXX`
/// widget, then
///
/// - implement `Declare` for `XXX`  mark `XXXBuilder` as its builder type.
/// - implement `DeclareBuilder` for `XXXBuilder` which build `XXX` and used by
///   `declare！` to build the `XXX` widget.
/// - for every field of `XXXBuilder`
///   - implement method with same name of the field and use to init the field.
///  [declare]: ../ribir/declare/index.html
#[proc_macro_derive(Declare, attributes(declare))]
pub fn declare_trait_macro_derive2(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  declare_derive::declare_derive(&mut input)
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

/// The macro use to declare a object, this macro will use `ctx!()` to access
/// the `BuildCtx`, so it can only use in the `fn_widget!` macro, or any scope
/// that called `set_build_ctx!` macro.
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
///     rdl!{ Row { wrap: true, child: rdl!{ Text { text: "hello" } } } }
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
  FnWidgetMacro::gen_code(input.into(), &mut DollarRefsCtx::top_level())
}

/// This macro just return the input token stream. It's do nothing but help
/// `ribir` mark that a macro has been expanded.
#[proc_macro]
pub fn ribir_expanded_ಠ_ಠ(input: TokenStream) -> TokenStream { input }

/// set the `BuildCtx` to a special variable `_ctx_ಠ_ಠ`, so the user can use
/// `ctx!` to access it.
#[proc_macro]
pub fn set_build_ctx(input: TokenStream) -> TokenStream {
  let input: proc_macro2::TokenStream = input.into();
  quote! { let _ctx_ಠ_ಠ = #input; }.into()
}

/// get the `BuildCtx` set by `set_build_ctx!` macro, if no `BuildCtx` set.
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
  PipeMacro::gen_code(input.into(), &mut DollarRefsCtx::top_level())
}

/// `watch!` macro use to convert a expression to a `Observable` stream. Use the
/// `$` mark the state reference and auto subscribe to its modify.
#[proc_macro]
pub fn watch(input: TokenStream) -> TokenStream {
  WatchMacro::gen_code(input.into(), &mut DollarRefsCtx::top_level())
}

/// macro split a new writer from a state writer. For example,
/// `split_writer!($label.visible);` will return a writer of `bool` that is
/// partial of the `Visibility`. This macros is a convenient way for
/// `StateWriter::split_writer`
#[proc_macro]
pub fn split_writer(input: TokenStream) -> TokenStream {
  writer_map_macro::gen_split_path_writer(input.into(), &mut DollarRefsCtx::top_level())
}

/// macro map a write to another state write. For example,
/// `map_writer!($label.visible);` will return a writer of `bool` that is
/// partial of the `Visibility`. This macros is a convenient way for
/// `StateWriter::map_writer`
#[proc_macro]
pub fn map_writer(input: TokenStream) -> TokenStream {
  writer_map_macro::gen_map_path_writer(input.into(), &mut DollarRefsCtx::top_level())
}

#[proc_macro]
pub fn include_svg(input: TokenStream) -> TokenStream {
  let w = parse_macro_input! { input as syn::LitStr };
  let mut span = proc_macro::Span::call_site();
  while let Some(p) = span.parent() {
    span = p;
  }
  let mut file = span.source_file().path();
  file.pop();
  file.push(w.value());
  let encoded_bytes = ribir_painter::Svg::open(file).and_then(|reader| reader.serialize());
  match encoded_bytes {
    Ok(data) => quote! {
      Svg::deserialize(#data).unwrap()
    }
    .into(),
    Err(err) => {
      let err = format!("{err}");
      quote! { compile_error!(#err)}.into()
    }
  }
}
