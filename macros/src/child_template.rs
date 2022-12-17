use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
  parse_quote, punctuated::Punctuated, token::Comma, AngleBracketedGenericArguments, Field, Fields,
  GenericArgument, PathArguments, PathSegment,
};

use crate::util::data_struct_unwrap;
const TML: &str = "Tml";
const ASSOCIATED_TEMPLATE: &str = "AssociatedTemplate";

pub(crate) fn derive_child_template(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: name, generics, data, .. } = input;
  let (g_impl, g_ty, g_where) = generics.split_for_impl();

  let stt = data_struct_unwrap(data, ASSOCIATED_TEMPLATE)?;
  let tml = Ident::new(&format!("{}{}", name, TML), name.span());

  let fields = match stt.fields {
    Fields::Named(ref named) => &named.named,
    Fields::Unit => {
      let err_str = format!(
        "Can't derive `{}` for a empty template.",
        ASSOCIATED_TEMPLATE
      );
      return Err(syn::Error::new(Span::call_site(), err_str));
    }
    Fields::Unnamed(_) => unreachable!(),
  };

  let mut init_values = quote! {};
  let mut fill_child_impl = quote! {};
  // builder define
  let builder_fields: Punctuated<Field, Comma> = fields
    .clone()
    .into_pairs()
    .map(|mut p| {
      let field = p.value();
      let field_name = field.ident.as_ref();
      let mut ty = &field.ty;

      let mut value = quote! { self.#field_name };
      if let Some(inner_ty) = option_type_extract(ty) {
        ty = inner_ty
      } else {
        let err = format!("Required child `{}` not specify", quote! { #ty });
        value.extend(quote! { .expect(#err)});
      };
      let punct = p.punct();
      init_values.extend(quote! { #field_name: #value #punct});

      fill_child_impl.extend(quote! {
        impl #g_impl FillTml<SelfImpl, #ty> for #tml #g_ty #g_where  {
          type New = Self;
          fn fill(mut self, c: #ty) -> Self {
            assert!(self.#field_name.is_none(), "Try to fill same type twice.");
            self.#field_name = Some(c);
            self
          }
        }
      });

      p.value_mut().ty = parse_quote!(Option<#ty>);
      p
    })
    .collect();

  let tokens = quote! {
    #[derive(Default)]
    #vis struct #tml #g_ty #g_where {
      #builder_fields
    }

    impl #g_impl Template for #name #g_ty #g_where {
      type Builder = #tml #g_ty;

      #[inline]
      fn builder() -> Self::Builder {  <_>::default() }
    }

    impl #g_impl TemplateBuilder for #tml #g_ty #g_where {
      type Target = #name #g_ty;
      #[inline]
      fn build_tml(self) -> Self::Target {#name { #init_values }}
    }


    impl #g_impl Declare for #name #g_ty #g_where {
      type Builder = #tml #g_ty;
      #[inline]
      fn declare_builder() -> Self::Builder { #name::builder() }
    }

    impl #g_impl #tml #g_ty {
      #[inline]
      #vis fn build(self, _: &BuildCtx) -> Self { self }
    }

    #fill_child_impl
  };

  Ok(tokens)
}

fn option_type_extract(ty: &syn::Type) -> Option<&syn::Type> {
  fn match_ident(seg: &PathSegment, ident: &str) -> bool {
    seg.ident == ident && seg.arguments.is_empty()
  }

  let syn::Type::Path(ref path) = ty else { return None };
  let mut iter = path.path.segments.iter().rev();
  iter
    .next()
    // the last segment must have and be `Option`
    .filter(|s| s.ident == "Option")
    .filter(|_| {
      // the second last can be None or "option"
      iter.next().map_or(true, |s| {
        match_ident(s, "option")
          && iter
            .next()
            // the second last can be None or "option" or "core"
            .map_or(true, |s| match_ident(s, "std") || match_ident(s, "core"))
      })
    })
    .and_then(|s| match &s.arguments {
      PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) => Some(args),
      _ => None,
    })
    .filter(|args| args.len() == 1)
    .and_then(|args| match args.first() {
      Some(GenericArgument::Type(ref ty)) => Some(ty),
      _ => None,
    })
}
