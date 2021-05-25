use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{
  spanned::Spanned,
  token::{Brace, Paren},
  Ident, Index, NestedMeta,
};

use crate::{attr_fields::pure_ident, widget_derive::ProxyDeriveInfo};

const STATEFUL_ATTR: &'static str = "stateful";
const STATE_ATTR_NAME: &'static str = "state";
const CUSTOM_IMPL: &'static str = "custom";

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

  let state_fields = info.attr_fields.attr_fields().into_iter().map(|(f, _)| f);
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
    .map(|name| prefix_ident("state_", &name));

  let state_ty = state_fields.clone().map(|f| &f.ty);
  let stateful_name = prefix_ident("Stateful", &quote! {#name});

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

  let stateful_def = if custom_impl_attr(attrs)? {
    quote! {
        #[derive(Widget)]
        #vis struct #stateful_name #w_ty_generics(StatefulImpl<#name
    #w_ty_generics>) #w_where_clause;   }
  } else {
    quote! {
        #[derive(Widget, RenderWidget, CombinationWidget)]
        #vis struct #stateful_name #w_ty_generics(#[proxy] StatefulImpl<#name
    #w_ty_generics>) #w_where_clause;   }
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

fn prefix_ident(prefix: &str, ident: &TokenStream2) -> Ident {
  syn::parse_str::<Ident>(&format!("{}{}", prefix, ident)).unwrap()
}

fn custom_impl_attr(attrs: Vec<NestedMeta>) -> Result<bool, TokenStream2> {
  let (custom_impl, errors): (Vec<_>, Vec<_>) = attrs.into_iter().partition(|attr| {
    if let NestedMeta::Meta(ref meta) = attr {
      if let syn::Meta::Path(path) = meta {
        return pure_ident(path, CUSTOM_IMPL);
      }
    }
    false
  });
  if !errors.is_empty() {
    let error_recursive = errors.into_iter().map(|attr| {
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
  } else {
    Ok(custom_impl.len() == 1)
  }
}
