use crate::util::prefix_ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{
  spanned::Spanned,
  token::{Brace, Paren},
  Ident, Index, NestedMeta,
};

use crate::{attr_fields::pure_ident, proxy_derive::ProxyDeriveInfo};

const STATEFUL_ATTR: &str = "stateful";
const STATE_ATTR_NAME: &str = "state";
const CUSTOM_IMPL: &str = "custom";
pub const STATE_PREFIX: &str = "state_";

pub(crate) fn stateful_derive(
  input: &mut syn::DeriveInput,
  attrs: Vec<NestedMeta>,
) -> Result<TokenStream2, TokenStream2> {
  let name = input.ident.clone();
  let vis = input.vis.clone();

  let info =
    ProxyDeriveInfo::new(input, STATEFUL_ATTR, STATE_ATTR_NAME)?.none_attr_specified_error()?;

  let state_name: Ident = syn::parse_str(&format!("{}State", name)).unwrap();

  let state_generis = info.attr_fields.attr_fields_generics();
  let (_, ty_generics, where_clause) = state_generis.split_for_impl();
  let (w_impl_generics, w_ty_generics, w_where_clause) = info.generics.split_for_impl();

  let state_fields = info.attr_fields.attr_fields().iter().map(|(f, _)| f);
  let state_field_names = state_fields.clone().enumerate().map(|(idx, f)| {
    f.ident.as_ref().map_or_else(
      || {
        let index = Index::from(idx);
        quote! {#index}
      },
      |ident| quote! {#ident},
    )
  });

  let state_fn_names = state_field_names
    .clone()
    .map(|name| prefix_ident(STATE_PREFIX, &name));

  let state_ty = state_fields.clone().map(|f| &f.ty);
  let mut stateful_name = prefix_ident("Stateful", &quote! {#name});

  // State define
  let mut state_def = quote! {
    #[derive(StatePartialEq)]
    #vis struct #state_name #ty_generics #where_clause
  };
  let to_surround = |tokens: &mut TokenStream2| {
    *tokens = quote! { #(#state_fields ,)* };
  };
  if info.attr_fields.is_tuple {
    Paren::default().surround(&mut state_def, to_surround);
    state_def = quote! {#state_def;};
  } else {
    Brace::default().surround(&mut state_def, to_surround)
  }

  // impl clone_states;
  let mut impl_clone_state = quote! { #state_name };
  let recurse_names = state_field_names.clone();
  if info.attr_fields.is_tuple {
    Paren::default().surround(&mut impl_clone_state, |tokens| {
      *tokens = quote! { #(self.#recurse_names.clone()) ,*};
    });
  } else {
    Brace::default().surround(&mut impl_clone_state, |tokens| {
      *tokens = quote! { #(#recurse_names: self.#recurse_names.clone()) ,*};
    });
  }

  let (custom_impl, custom_name) = custom_impl_attr(attrs)?;
  if let Some(custom_name) = custom_name {
    stateful_name = custom_name;
  }
  let stateful_def = if custom_impl {
    quote! {
      #vis struct #stateful_name #w_ty_generics(
        StatefulImpl<#name #w_ty_generics>) #w_where_clause;
    }
  } else {
    quote! {
      #[derive(RenderWidget, CombinationWidget, SingleChildWidget, MultiChildWidget)]
      #vis struct #stateful_name #w_ty_generics(
        #[proxy] StatefulImpl<#name #w_ty_generics>
      ) #w_where_clause;
    }
  };

  let expanded = quote! {
    // Define custom state.
    #state_def

    // A stateful version widget
    #stateful_def

    // Every state have a state observable.
    impl #w_impl_generics #stateful_name #w_ty_generics #w_where_clause {
      #(
        pub fn #state_fn_names(&mut self)
          -> impl LocalObservable<'static, Item = StateChange<#state_ty>, Err = ()> {
          self.0.state_change(|w| w.#state_field_names.clone())
        }
      )*
    }

    impl #w_impl_generics Stateful for #stateful_name #w_ty_generics #w_where_clause {
      type RawWidget = #name #w_ty_generics;
      fn ref_cell(&self) -> StateRefCell<Self::RawWidget> {
        self.0.ref_cell()
      }
    }

    impl #w_impl_generics CloneStates for #name #w_ty_generics #w_where_clause {
      type States =  #state_name #ty_generics;
      fn clone_states(&self) -> Self::States {
        #impl_clone_state
      }
    }

    impl #w_impl_generics IntoStateful for #name #w_ty_generics #w_where_clause {
      type S = #stateful_name #w_ty_generics;

      #[inline]
      fn into_stateful(self) -> Self::S {
        #stateful_name(StatefulImpl::new(self))
      }
    }

    impl #w_impl_generics !NoAttrs for #stateful_name #w_ty_generics #w_where_clause {}

    impl #w_impl_generics AttachAttr for #stateful_name #w_ty_generics #w_where_clause {
      type W = Self;
      #[inline]
      fn into_attr_widget(self) -> Self::W { self }
    }

    impl #w_impl_generics AttrsAccess for #stateful_name #w_ty_generics #w_where_clause {
      #[inline]
      fn get_attrs(&self) -> Option<AttrRef<Attributes>> { self.0.get_attrs() }

      #[inline]
      fn get_attrs_mut(&mut self) -> Option<AttrRefMut<Attributes>> { self.0.get_attrs_mut() }
    }

    impl #w_impl_generics Attrs for #stateful_name #w_ty_generics #w_where_clause {
      #[inline]
      fn attrs(&self) -> AttrRef<Attributes> { self.0.attrs() }

      #[inline]
      fn attrs_mut(&mut self) -> AttrRefMut<Attributes> { self.0.attrs_mut() }
    }

    impl #w_impl_generics std::ops::Deref for #stateful_name #w_ty_generics #w_where_clause {
      type Target = StatefulImpl<#name #w_ty_generics>;
      #[inline]
      fn deref(&self) -> &Self::Target { &self.0}
    }

    impl #w_impl_generics std::ops::DerefMut for #stateful_name #w_ty_generics #w_where_clause {
      #[inline]
      fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
  };

  Ok(quote! {
    #input

    #expanded
  })
}

fn custom_impl_attr(attrs: Vec<NestedMeta>) -> Result<(bool, Option<Ident>), TokenStream2> {
  let (custom_impl, not_supports): (Vec<_>, Vec<_>) = attrs.into_iter().partition(|attr| {
    if let NestedMeta::Meta(meta) = attr {
      match meta {
        syn::Meta::Path(path) => pure_ident(path, CUSTOM_IMPL),
        syn::Meta::NameValue(meta) => pure_ident(&meta.path, CUSTOM_IMPL),
        _ => false,
      }
    } else {
      false
    }
  });
  if !not_supports.is_empty() {
    let error_recursive = not_supports.into_iter().map(|attr| {
      let err_str = format!("#[{}] not support this argument", STATEFUL_ATTR);
      quote_spanned! {  attr.span() => compile_error!(#err_str); }
    });
    Err(quote! { #(#error_recursive)* })
  } else if custom_impl.len() > 1 {
    let size = custom_impl.len();

    let error_recursive = custom_impl.into_iter().enumerate().map(|(idx, attr)| {
      let err_str = format!(
        "{}/{}: too many {} for {}, only one need",
        idx + 1,
        size,
        CUSTOM_IMPL,
        STATEFUL_ATTR
      );
      quote_spanned! { attr.span() => compile_error!(#err_str); }
    });
    Err(quote! { #(#error_recursive)* })
  } else if let Some(attr) = custom_impl.get(0) {
    match attr {
      NestedMeta::Meta(syn::Meta::Path(_)) => Ok((true, None)),
      NestedMeta::Meta(syn::Meta::NameValue(named)) => match named.lit {
        syn::Lit::Str(ref name) => match name.parse::<Ident>() {
          Ok(name) => Ok((true, Some(name))),
          Err(_) => {
            let err_str = format!("{} is not a valid name for widget.", name.value());
            Err(quote_spanned! {  named.lit.span() => compile_error!(#err_str); })
          }
        },
        _ => {
          let err_str = "Only a valid string can be used as widget name.";
          Err(quote_spanned! {  named.lit.span() => compile_error!(#err_str); })
        }
      },
      _ => unreachable!(),
    }
  } else {
    Ok((false, None))
  }
}
