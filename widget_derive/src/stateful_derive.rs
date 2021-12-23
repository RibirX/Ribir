use crate::util::prefix_ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, NestedMeta};

use crate::attr_fields::pure_ident;

const STATEFUL_ATTR: &str = "stateful";
const CUSTOM_IMPL: &str = "custom";

pub(crate) fn stateful_derive(
  input: &mut syn::DeriveInput,
  attrs: Vec<NestedMeta>,
) -> Result<TokenStream2, TokenStream2> {
  let DeriveInput { ident: name, generics, vis, .. } = input;

  let (w_impl_generics, w_ty_generics, w_where_clause) = generics.split_for_impl();
  let stateful_name = prefix_ident("Stateful", &quote! {#name});

  let custom_impl = custom_impl_attr(attrs)?;

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
    // A stateful version widget
    #stateful_def


    impl #w_impl_generics Stateful for #stateful_name #w_ty_generics #w_where_clause {
      type RawWidget = #name #w_ty_generics;
      fn state_ref(&self) -> StateRef<Self::RawWidget> {
        self.0.state_ref()
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
      fn get_attrs(&self) -> Option<&Attributes> { self.0.get_attrs() }

      #[inline]
      fn get_attrs_mut(&mut self) -> Option<&mut Attributes> { self.0.get_attrs_mut() }
    }

    impl #w_impl_generics Attrs for #stateful_name #w_ty_generics #w_where_clause {
      #[inline]
      fn attrs(&self) -> &Attributes { self.0.attrs() }

      #[inline]
      fn attrs_mut(&mut self) -> &mut Attributes { self.0.attrs_mut() }
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

fn custom_impl_attr(attrs: Vec<NestedMeta>) -> Result<bool, TokenStream2> {
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
  } else {
    Ok(custom_impl.len() == 1)
  }
}
