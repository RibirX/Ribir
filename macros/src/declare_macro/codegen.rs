use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Ident, Visibility, spanned::Spanned};

use super::ir::{DeclareField, Declarer};

pub(crate) struct CodegenContext<'a> {
  pub declarer: &'a Declarer<'a>,
  pub vis: &'a Visibility,
  pub host: &'a Ident,
  pub g_impl: syn::ImplGenerics<'a>,
  pub g_ty: syn::TypeGenerics<'a>,
  pub g_where: Option<&'a syn::WhereClause>,
}

impl<'a> CodegenContext<'a> {
  pub fn new(declarer: &'a Declarer<'a>) -> Self {
    let (g_impl, g_ty, g_where) = declarer.original.generics.split_for_impl();
    Self {
      declarer,
      vis: &declarer.original.vis,
      host: &declarer.original.ident,
      g_impl,
      g_ty,
      g_where,
    }
  }

  // ===== Main generation method =====

  pub fn generate(&self) -> TokenStream {
    let mut tokens = TokenStream::new();
    tokens.extend(self.gen_declare_struct());
    tokens.extend(self.gen_declare_trait_impl());
    tokens.extend(self.gen_impl_block());
    tokens.extend(self.gen_obj_declarer_impl());
    if let Some(target) = self.deref_target_type() {
      tokens.extend(self.gen_deref_impls(target));
    }
    tokens.extend(self.gen_widget_macro());
    tokens
  }

  // ===== Struct generation =====

  fn gen_declare_struct(&self) -> TokenStream {
    let Declarer { name, original, .. } = self.declarer;
    let vis = self.vis;
    let generics = &original.generics;
    let host = self.host;

    let (wrapper_field_def, _) = self.wrapper_field();

    let marker_field = if self.declarer.needs_marker() {
      let g_ty = &self.g_ty;
      quote! { _marker: std::marker::PhantomData<#host #g_ty>, }
    } else {
      quote! {}
    };

    let builder_field_attr = self.builder_field_attr();
    let builder_fields = self.declarer.no_skip_fields().filter_map(|f| {
      let member = f.member();
      let ty = self.builder_storage_ty(f)?;
      let attr = builder_field_attr.as_ref();
      Some(quote! {
        #attr
        #member: Option<#ty>
      })
    });

    let g_where = self.g_where;

    quote! {
      #vis struct #name #generics #g_where {
        #wrapper_field_def
        #marker_field
        #(#builder_fields,)*
      }
    }
  }

  // ===== Trait implementations =====

  fn gen_declare_trait_impl(&self) -> TokenStream {
    let name = &self.declarer.name;
    let host = self.host;
    let g_impl = &self.g_impl;
    let g_ty = &self.g_ty;
    let g_where = self.g_where;

    let (_, wrapper_field_init) = self.wrapper_field();

    let marker_init = if self.declarer.needs_marker() {
      quote! { _marker: std::marker::PhantomData, }
    } else {
      quote! {}
    };

    let field_inits = self.declarer.no_skip_fields().filter_map(|f| {
      let member = f.member();
      self.builder_storage_ty(f)?;
      Some(quote! { #member: None })
    });

    quote! {
      impl #g_impl Declare for #host #g_ty #g_where {
        type Builder = #name #g_ty;

        fn declarer() -> Self::Builder {
          #name {
            #wrapper_field_init
            #marker_init
            #(#field_inits,)*
          }
        }
      }
    }
  }

  fn gen_obj_declarer_impl(&self) -> TokenStream {
    let name = &self.declarer.name;
    let g_impl = &self.g_impl;
    let g_ty = &self.g_ty;
    let g_where = self.g_where;
    let declarer = self.declarer;
    let host = declarer.host();

    // Step 1: Determine basic properties
    let self_mut = if declarer.eager { quote!() } else { quote!(mut) };
    let required_checks: Vec<_> =
      if declarer.eager { self.required_field_checks().collect() } else { vec![] };

    let has_fields = !declarer.original.fields.is_empty();
    let needs_stateful = !declarer.stateless && (declarer.eager || has_fields);
    let needs_fat = !declarer.simple;

    // Step 2: Build target type by composition
    let target = self.wrap_target_type(quote! { #host #g_ty }, needs_stateful, needs_fat);

    // Step 3: Build finish body
    let finish_body = if !declarer.eager && needs_fat && needs_stateful {
      // Special case: lazy mode with full pipe/event handling
      self.gen_full_stateful_finish()
    } else if declarer.eager {
      // Eager mode: return stored value directly
      if declarer.simple && declarer.stateless {
        self.build_widget_simple()
      } else if declarer.simple {
        quote! { self.inner }
      } else {
        quote! { self.fat_ಠ_ಠ }
      }
    } else {
      // Lazy mode: build and wrap
      let mut obj = self.build_widget_simple();
      if needs_stateful {
        obj = quote! { Stateful::new(#obj) };
      }
      if needs_fat {
        obj = quote! { self.fat_ಠ_ಠ.map(|_| #obj) };
      }
      obj
    };

    quote! {
      impl #g_impl ObjDeclarer for #name #g_ty #g_where {
        type Target = #target;

        #[track_caller]
        fn finish(#self_mut self) -> Self::Target {
          #(#required_checks)*
          #finish_body
        }
      }
    }
  }

  // ===== Impl block =====

  fn gen_impl_block(&self) -> TokenStream {
    let name = &self.declarer.name;
    let g_impl = &self.g_impl;
    let g_ty = &self.g_ty;
    let g_where = self.g_where;

    let extra_methods = self.gen_extra_methods();
    let methods = self.gen_setter_methods();

    quote! {
      impl #g_impl #name #g_ty #g_where {
        #extra_methods
        #methods
      }
    }
  }

  fn gen_setter_methods(&self) -> TokenStream {
    self
      .declarer
      .fields
      .iter()
      .filter(|f| f.need_set_method())
      .map(|f| self.gen_setter(f))
      .collect()
  }

  fn gen_setter(&self, f: &DeclareField<'_>) -> TokenStream {
    let d = self.declarer;
    let field_name = f.member();
    let doc = f.doc_attr();
    let set_method = f.set_method_name();
    let ty = &f.field.ty;
    let vis = self.vis;

    // Check if this field needs storage (eager mode without default doesn't need
    // storage)
    let needs_storage = self.builder_storage_ty(f).is_some();

    // All setter variants produce: generics, param_ty, body
    let (generics, param_ty, body) = match (d.eager, d.simple, d.stateless) {
      // Simple: just store the value
      (true, true, true) | (false, true, _) | (false, _, true) => {
        if f.is_strict() {
          (quote!(), quote!(#ty), quote! { self.#field_name = Some(v); })
        } else {
          (quote!(), quote!(impl Into<#ty>), quote! { self.#field_name = Some(v.into()); })
        }
      }

      // Eager stateful: write to host, store marker only if needed
      (true, true, false) => {
        let host = quote! { self.inner };
        let set_logic = self.gen_setter_logic(f, quote! { #host.write() }, quote! { v });
        let store = if needs_storage {
          quote! { self.#field_name = Some(()); }
        } else {
          quote!()
        };
        if f.is_strict() {
          (
            quote!(),
            quote!(#ty),
            quote! {
              #set_logic
              #store
            },
          )
        } else {
          (
            quote!(<_K: ?Sized>),
            quote!(impl RInto<#ty, _K>),
            quote! {
              let v = v.r_into();
              #set_logic
              #store
            },
          )
        }
      }

      // Full: pipe/event handling via FatObj
      (is_eager, false, is_stateless) => {
        let wrapper = quote! { self.fat_ಠ_ಠ };
        let (generics, param_ty, value_expr) = self.gen_pipe_setter_param(f);
        let set_logic = if f.event_meta().is_some() {
          self.gen_event_set_logic(f, is_eager, &wrapper, &value_expr)
        } else {
          self.gen_non_event_set_logic(f, is_eager, is_stateless, &wrapper, &value_expr)
        };
        let field_store = if is_eager && needs_storage {
          quote! { self.#field_name = Some(()); }
        } else {
          quote!()
        };
        (
          generics,
          param_ty,
          quote! {
            #set_logic
            #field_store
          },
        )
      }
    };

    quote! {
      #[inline]
      #[allow(clippy::type_complexity)]
      #doc
      #vis fn #set_method #generics (&mut self, v: #param_ty) -> &mut Self {
        #body
        self
      }
    }
  }

  fn gen_extra_methods(&self) -> Option<TokenStream> {
    let declarer = self.declarer;
    if !declarer.eager || declarer.stateless {
      return None;
    }

    let access = if declarer.simple {
      quote! { &self.inner }
    } else {
      quote! { self.fat_ಠ_ಠ.host() }
    };

    let host = self.host;
    let g_ty = &self.g_ty;
    let vis = self.vis;

    Some(quote! {
      /// Returns a reference to the inner stateful widget for partial setters.
      #[inline]
      #vis fn host(&self) -> &Stateful<#host #g_ty> {
        #access
      }
    })
  }

  // ===== Auxiliary implementations =====

  fn gen_deref_impls(&self, target: TokenStream) -> TokenStream {
    let name = &self.declarer.name;
    let g_impl = &self.g_impl;
    let g_ty = &self.g_ty;
    let g_where = self.g_where;

    quote! {
      impl #g_impl std::ops::Deref for #name #g_ty #g_where {
        type Target = #target;
        #[inline]
        fn deref(&self) -> &Self::Target {
          &self.fat_ಠ_ಠ
        }
      }

      impl #g_impl std::ops::DerefMut for #name #g_ty #g_where {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
          &mut self.fat_ಠ_ಠ
        }
      }
    }
  }

  fn gen_widget_macro(&self) -> TokenStream {
    let name = self.host;
    let vis = self.vis;
    let macro_name = name.to_string().to_snake_case();
    let doc = format!(
      "Macro used to generate a function widget using `{}` as the root widget.",
      macro_name
    );
    let macro_name = Ident::new(&macro_name, name.span());
    let export_attr = if matches!(vis, Visibility::Public(_)) {
      quote! { #[macro_export] }
    } else {
      quote! { #[allow(unused_macros)] }
    };
    quote! {
      #[allow(unused_macros)]
      #export_attr
      #[doc = #doc]
      macro_rules! #macro_name {
        ($($t: tt)*) => {
          fn_widget! { @ #name { $($t)* } }
        };
      }
      #[allow(unused_imports)]
      #vis use #macro_name;
    }
  }

  // ===== Mode-related types/fragments =====

  /// Returns the target type for ObjDeclarer::Target
  fn target_type(&self) -> TokenStream {
    let declarer = self.declarer;
    let host = declarer.host();
    let g_ty = &self.g_ty;
    self.wrap_target_type(quote! { #host #g_ty }, !declarer.stateless, !declarer.simple)
  }

  /// Returns (field_def, field_init) for the wrapper field based on mode
  fn wrapper_field(&self) -> (TokenStream, TokenStream) {
    let declarer = self.declarer;
    let host = declarer.host();
    let g_ty = &self.g_ty;

    // Determine field name, type, and init value based on mode
    match (declarer.simple, declarer.stateless, declarer.eager) {
      // No wrapper field needed
      (true, true, _) | (true, _, false) => (quote!(), quote!()),
      // FatObj wrapper
      (false, stateless, eager) => {
        let inner_ty = if eager {
          quote! { #host #g_ty }
        } else {
          quote! { () }
        };
        let inner_init = if eager {
          self.gen_eager_default_expr()
        } else {
          quote! { () }
        };
        let (ty, init) = if stateless || !eager {
          (inner_ty, inner_init)
        } else {
          (quote! { Stateful<#inner_ty> }, quote! { Stateful::new(#inner_init) })
        };
        (quote! { fat_ಠ_ಠ: FatObj<#ty>, }, quote! { fat_ಠ_ಠ: FatObj::new(#init), })
      }
      // Simple stateful eager: inner field
      (true, false, true) => {
        let init = self.gen_eager_default_expr();
        (quote! { inner: Stateful<#host #g_ty>, }, quote! { inner: Stateful::new(#init), })
      }
    }
  }

  /// Returns the storage type for a builder field based on the current mode
  fn builder_storage_ty(&self, f: &DeclareField<'_>) -> Option<TokenStream> {
    let declarer = self.declarer;
    if declarer.eager {
      if declarer.simple && declarer.stateless {
        let ty = &f.field.ty;
        Some(quote! { #ty })
      } else if f.default_value().is_some() {
        // Eager mode with default: no storage needed
        None
      } else {
        // Eager mode without default: store marker for required check
        Some(quote! { () })
      }
    } else {
      Some(self.builder_field_ty(f))
    }
  }

  /// Returns the deref target type for Deref/DerefMut implementations, or None
  /// if not applicable
  fn deref_target_type(&self) -> Option<TokenStream> {
    let declarer = self.declarer;
    match (declarer.simple, declarer.eager) {
      (true, _) => None,
      (false, true) => Some(self.target_type()),
      (false, false) => Some(quote! { FatObj<()> }),
    }
  }

  // ===== Field-level helpers =====

  fn gen_setter_logic(
    &self, f: &DeclareField<'_>, writer: TokenStream, value: TokenStream,
  ) -> TokenStream {
    if let Some(setter) = f.setter_name() {
      if let Some(setter_ty) = f.setter_ty() {
        quote! {
          let v: #setter_ty = #value.into();
          #writer.#setter(v);
        }
      } else {
        quote! { #writer.#setter(#value); }
      }
    } else {
      let member = f.member();
      quote! { #writer.#member = #value; }
    }
  }

  fn field_eager_default(&self, f: &DeclareField<'_>) -> TokenStream {
    f.default_value()
      .unwrap_or_else(|| quote! { <_>::default() })
  }

  // ===== Private helper methods =====

  fn builder_field_attr(&self) -> Option<TokenStream> {
    if self.declarer.eager { None } else { Some(quote! { #[allow(clippy::type_complexity)] }) }
  }

  fn builder_field_ty(&self, f: &DeclareField<'_>) -> TokenStream {
    let declarer = self.declarer;
    let ty = &f.field.ty;
    if declarer.simple || declarer.stateless {
      quote! { #ty }
    } else {
      quote! { PipeValue<#ty> }
    }
  }

  fn gen_eager_default_expr(&self) -> TokenStream {
    let declarer = self.declarer;
    let host = declarer.host();

    match &declarer.original.fields {
      syn::Fields::Named(_) => {
        let field_inits = declarer.fields.iter().map(|f| {
          let name = f.member();
          let v = self.field_eager_default(f);
          quote! { #name: #v }
        });
        quote! { #host { #(#field_inits),* } }
      }
      syn::Fields::Unnamed(_) => {
        let values = declarer
          .fields
          .iter()
          .map(|f| self.field_eager_default(f));
        quote! { #host(#(#values),*) }
      }
      syn::Fields::Unit => quote! { #host },
    }
  }

  fn wrap_target_type(
    &self, base: TokenStream, needs_stateful: bool, needs_fat: bool,
  ) -> TokenStream {
    let mut wrapped = base;
    if needs_stateful {
      wrapped = quote! { Stateful<#wrapped> };
    }
    if needs_fat {
      wrapped = quote! { FatObj<#wrapped> };
    }
    wrapped
  }

  fn required_field_error(host: &Ident, field_name: &Ident) -> String {
    format!("Required field `{host}::{field_name}` not set")
  }

  fn build_widget(&self, values: impl Iterator<Item = TokenStream>) -> TokenStream {
    let host = self.declarer.host();
    let finish_obj = match &self.declarer.original.fields {
      syn::Fields::Named(_) => {
        let members = self.declarer.all_members();
        quote!(#host { #(#members: #values),* })
      }
      syn::Fields::Unnamed(_) => quote!(#host(#(#values),*)),
      syn::Fields::Unit => quote!(#host),
    };
    if let Some(validate) = self.declarer.validate.as_ref() {
      quote! { #finish_obj.#validate().expect("Validation failed") }
    } else {
      finish_obj
    }
  }

  fn build_widget_simple(&self) -> TokenStream { self.build_widget(self.finish_values_simple()) }

  fn finish_values_simple(&'a self) -> impl Iterator<Item = TokenStream> + 'a {
    let host = self.declarer.host();
    self.declarer.fields.iter().map(move |f| {
      let field_name = f.member();

      if f.is_not_skip() {
        match f.default_value() {
          Some(default) => quote! { self.#field_name.take().unwrap_or_else(|| #default) },
          None => {
            let err_msg = Self::required_field_error(host, field_name);
            quote_spanned! { field_name.span() => self.#field_name.take().expect(#err_msg) }
          }
        }
      } else {
        // skip field always has default value
        f.default_value().unwrap()
      }
    })
  }

  fn required_field_checks(&'a self) -> impl Iterator<Item = TokenStream> + 'a {
    let host = self.declarer.host();
    self
      .declarer
      .no_skip_fields()
      .filter(|f| f.default_value().is_none())
      .map(move |f| {
        let field_name = f.member();
        let err_msg = Self::required_field_error(host, field_name);
        quote_spanned! { field_name.span() =>
          if self.#field_name.is_none() {
            panic!(#err_msg);
          }
        }
      })
  }

  fn field_values_full(&'a self) -> impl Iterator<Item = TokenStream> + 'a {
    let host = self.declarer.host();
    self.declarer.fields.iter().map(move |f| {
      let field_name = f.member();
      let ty = &f.field.ty;

      let value_expr = if f.is_not_skip() {
        match f.default_value() {
          Some(default) => quote! {
            Option::take(&mut self.#field_name).map_or_else(|| (#default, None), |v| v.unzip())
          },
          None => {
            let err_msg = Self::required_field_error(host, field_name);
            quote! { Option::take(&mut self.#field_name).expect(#err_msg).unzip() }
          }
        }
      } else {
        // skip field always has default value
        let default = f.default_value().unwrap();
        quote! { (#default, None) }
      };

      quote_spanned! { f.field.span() =>
        #[allow(clippy::type_complexity)]
        let #field_name: (#ty, Option<ValueStream<#ty>>) = #value_expr;
      }
    })
  }

  fn gen_full_stateful_finish(&self) -> TokenStream {
    let field_names: Vec<_> = self.declarer.all_members().collect();
    let field_values = self.field_values_full();
    let finish_obj = self.build_widget(field_names.iter().map(|m| quote! {#m.0}));

    let (field_tys, setter_logic): (Vec<_>, Vec<_>) = self
      .declarer
      .fields
      .iter()
      .map(|f| (&f.field.ty, self.gen_setter_logic(f, quote! { this_ಠ_ಠ.write() }, quote! { v })))
      .unzip();

    let event_bindings: Vec<_> = self
      .declarer
      .fields
      .iter()
      .filter_map(|f| {
        let event_meta = f.event_meta()?;
        let field_name = f.member();
        let event_type = &event_meta.event_type;
        let set_logic = self.gen_setter_logic(f, quote! { this_ಠ_ಠ.write() }, quote! { v });

        let convert_expr = self.gen_convert_expr(f, &set_logic);

        Some((field_name.clone(), event_type.clone(), convert_expr))
      })
      .collect();

    let event_field_names: Vec<_> = event_bindings.iter().map(|(f, _, _)| f).collect();
    let event_types: Vec<_> = event_bindings.iter().map(|(_, t, _)| t).collect();
    let event_convert_exprs: Vec<_> = event_bindings.iter().map(|(_, _, e)| e).collect();

    quote! {
      #(#field_values)*
      let _obj_ಠ_ಠ = #finish_obj;
      let this_ಠ_ಠ = Stateful::new(_obj_ಠ_ಠ);

      let mut fat_ಠ_ಠ = self.fat_ಠ_ಠ;

      #(
        if #event_field_names.1.is_none() {
          let this_ಠ_ಠ = this_ಠ_ಠ.clone_writer();
          fat_ಠ_ಠ.on_custom::<#event_types>(move |e: &mut CustomEvent<#event_types>| {
            #event_convert_exprs
          });
        }
      )*

      #(
        if let Some(o) = #field_names.1 {
          let this_ಠ_ಠ = this_ಠ_ಠ.clone_writer();
          let u = o.subscribe(move |v: #field_tys| { #setter_logic });
          fat_ಠ_ಠ.on_disposed(move |_| u.unsubscribe());
        }
      )*

      fat_ಠ_ಠ.map(move |_| this_ಠ_ಠ)
    }
  }

  fn gen_convert_expr(&self, f: &DeclareField<'_>, set_logic: &TokenStream) -> TokenStream {
    let event_meta = f.event_meta().expect("Event meta required");
    let field_ty = &f.field.ty;

    if let Some(chain) = &event_meta.convert_chain {
      quote! {
        let v: Option<#field_ty> = e.data()#chain.into();
        if let Some(v) = v {
          #set_logic
        }
      }
    } else {
      quote! {
        let v: #field_ty = e.data().clone().into();
        #set_logic
      }
    }
  }

  fn gen_event_set_logic(
    &self, field: &DeclareField<'_>, is_eager: bool, wrapper: &TokenStream, v_val: &TokenStream,
  ) -> TokenStream {
    let event_meta = field.event_meta().expect("Event meta required");
    let event_type = &event_meta.event_type;
    let field_name = field.member();
    let set_logic = quote! { *writer.write() = v; };
    let convert_expr = self.gen_convert_expr(field, &set_logic);

    // Common parts for both branches
    let two_way_value = quote! {
      PipeValue::Pipe {
        init_value: writer.read().clone(),
        pipe
      }
    };

    let (pipe_arm, two_way_store, event_target) = if is_eager {
      let init_pipe =
        self.gen_eager_init_sub_widget(wrapper, false, quote! { pipe_value }, field_name);
      let init_two_way = self.gen_eager_init_sub_widget(wrapper, false, two_way_value, field_name);
      (init_pipe, init_two_way, quote! { #wrapper })
    } else {
      (
        quote! { self.#field_name = Some(pipe_value); },
        quote! { self.#field_name = Some(#two_way_value); },
        quote! { self },
      )
    };

    quote! {
      match #v_val {
        TwoWayValue::Pipe(pipe_value) => { #pipe_arm }
        TwoWayValue::TwoWay(writer) => {
          let pipe = Pipe::from_watcher(writer.clone_watcher());
          #two_way_store
          #event_target.on_custom::<#event_type>(move |e: &mut CustomEvent<#event_type>| {
            #convert_expr
          });
        }
      }
    }
  }

  fn gen_non_event_set_logic(
    &self, f: &DeclareField<'_>, is_eager: bool, is_stateless: bool, wrapper: &TokenStream,
    v_val: &TokenStream,
  ) -> TokenStream {
    let field_name = f.member();

    // Lazy modes: store the value directly
    if !is_eager {
      return quote! { self.#field_name = Some(#v_val); };
    }

    // Eager modes: use init_sub_widget
    self.gen_eager_init_sub_widget(wrapper, is_stateless, quote! { #v_val }, field_name)
  }

  fn gen_eager_init_sub_widget(
    &self, wrapper: &TokenStream, is_stateless: bool, value: TokenStream, field_name: &Ident,
  ) -> TokenStream {
    if is_stateless {
      quote! {
        let mix = #wrapper.mix_builtin_widget();
        mix.init_sub_widget(#value, &#wrapper, |w, v| w.#field_name = v);
      }
    } else {
      quote! {
        let host = #wrapper.host().clone_writer();
        let mix = #wrapper.mix_builtin_widget();
        mix.init_sub_widget(#value, &host, |w, v| w.#field_name = v);
      }
    }
  }

  fn gen_pipe_setter_param(&self, f: &DeclareField<'_>) -> (TokenStream, TokenStream, TokenStream) {
    let ty = &f.field.ty;
    match (f.event_meta(), f.is_strict()) {
      (Some(_), true) => (quote!(), quote!(TwoWayValue<#ty>), quote!(v)),
      (Some(_), false) => {
        (quote!(<_K: ?Sized>), quote!(impl RInto<TwoWayValue<#ty>, _K>), quote!(v.r_into()))
      }
      (None, true) => (quote!(), quote!(#ty), quote!(PipeValue::Value(v))),
      (None, false) => {
        (quote!(<_K: ?Sized>), quote!(impl RInto<PipeValue<#ty>, _K>), quote!(v.r_into()))
      }
    }
  }
}
