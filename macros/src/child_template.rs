use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
  parse_quote, punctuated::Pair, spanned::Spanned, token::Comma, AngleBracketedGenericArguments,
  DataEnum, Field, Fields, FieldsNamed, FieldsUnnamed, GenericArgument, Index, PathArguments,
  PathSegment,
};

const TML: &str = "Tml";
const ASSOCIATED_TEMPLATE: &str = "AssociatedTemplate";

pub(crate) fn derive_child_template(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: name, generics, data, .. } = input;
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let tml = Ident::new(&format!("{name}{TML}"), name.span());

  let mut tokens = quote! {
    impl #g_impl Template for #name #g_ty #g_where {
      type Builder = #tml #g_ty;

      #[inline]
      fn builder() -> Self::Builder {  <_>::default() }
    }

    impl #g_impl Declare for #name #g_ty #g_where {
      type Builder = #tml #g_ty;
      #[inline]
      fn declare_builder() -> Self::Builder { #name::builder() }
    }

    impl #g_impl DeclareBuilder for #tml #g_ty {
      type Target = Self;
      #[inline]
      fn build(self, _: &BuildCtx) -> Self { self }
    }
  };

  let fill_tml_impl = |field_name: TokenStream, ty: &syn::Type, tokens: &mut TokenStream| {
    tokens.extend(quote! {
      impl #g_impl FillTml<SelfImpl, #ty> for #tml #g_ty #g_where  {
        fn fill(&mut self, c: #ty) {
          assert!(self.#field_name.is_none(), "Try to fill same type twice.");
          self.#field_name = Some(c);
        }
      }
    });
  };
  match data {
    syn::Data::Struct(stt) => match &stt.fields {
      Fields::Named(FieldsNamed { named: fields, .. }) => {
        let builder_fields = fields.clone().into_pairs().map(convert_to_builder_pair);
        tokens.extend(quote! {
          #[derive(Default)]
          #vis struct #tml #g_ty #g_where {
            #(#builder_fields)*
          }
        });

        let init_values = fields.iter().map(|field| {
          let field_name = field.ident.as_ref().unwrap();
          let ty = &field.ty;
          let value = gen_init_value_tokens(quote!(#field_name), ty);
          quote! {#field_name: #value}
        });

        tokens.extend(quote! {
          impl #g_impl TemplateBuilder for #tml #g_ty #g_where {
            type Target = #name #g_ty;
            #[inline]
            fn build_tml(self) -> Self::Target {#name { #(#init_values),* }}
          }
        });
        fields.iter().for_each(|f| {
          let f_name = f.ident.as_ref().unwrap();
          let ty = option_type_extract(&f.ty).unwrap_or(&f.ty);
          fill_tml_impl(quote! {#f_name}, ty, &mut tokens)
        });
      }
      Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => {
        let builder_fields = fields.clone().into_pairs().map(convert_to_builder_pair);
        tokens.extend(quote! {
          #[derive(Default)]
          #vis struct #tml #g_ty #g_where(#(#builder_fields)*);
        });

        let init_values = fields.iter().enumerate().map(|(idx, field)| {
          let idx = Index::from(idx);
          gen_init_value_tokens(quote!(#idx), &field.ty)
        });

        tokens.extend(quote! {
          impl #g_impl TemplateBuilder for #tml #g_ty #g_where {
            type Target = #name #g_ty;
            #[inline]
            fn build_tml(self) -> Self::Target {#name(#(#init_values),* ) }
          }
        });

        fields.iter().enumerate().for_each(|(idx, f)| {
          let ty = option_type_extract(&f.ty).unwrap_or(&f.ty);
          let idx = Index::from(idx);
          fill_tml_impl(quote! {#idx}, ty, &mut tokens)
        });
      }
      Fields::Unit => {
        let err_str = format!("Can't derive `{ASSOCIATED_TEMPLATE}` for a empty template.",);
        return Err(syn::Error::new(Span::call_site(), err_str));
      }
    },
    syn::Data::Enum(DataEnum { variants, .. }) => {
      let err_str = format!("Child `{}` not specify.", quote! { #name });
      tokens.extend(quote! {
        #[derive(Default)]
        #vis struct #tml #g_ty #g_where(Option<#name>);

        impl #g_impl TemplateBuilder for #tml #g_ty #g_where {
          type Target = #name #g_ty;
          #[inline]
          fn build_tml(self) -> Self::Target {
            self.0.expect(&#err_str)
          }
        }
      });

      variants.iter().for_each(|v| {
        if let Fields::Unnamed(FieldsUnnamed { unnamed, .. }) = &v.fields {
          // only the enum variant has a single type need to implement fill convert.
          if unnamed.len() == 1 {
            let f = unnamed.first().unwrap();
            let ty = &f.ty;
            let v_name = &v.ident;
            tokens.extend(quote! {
              impl #g_impl FillTml<SelfImpl, #ty> for #tml #g_ty #g_where  {
                fn fill(&mut self, c: #ty) {
                  assert!(self.0.is_none(), "Try to fill enum template with two variant.");
                  self.0 = Some(#name::#v_name(c));
                }
              }
            });
          }
        }
      });
    }
    syn::Data::Union(u) => {
      let err_str = format!("`{ASSOCIATED_TEMPLATE}` not support for Union");
      return Err(syn::Error::new(u.union_token.span(), err_str));
    }
  }

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

fn convert_to_builder_pair(mut p: Pair<Field, Comma>) -> Pair<Field, Comma> {
  let ty = &p.value().ty;
  if option_type_extract(ty).is_none() {
    p.value_mut().ty = parse_quote!(Option<#ty>);
  };
  p
}

fn gen_init_value_tokens(field_name: TokenStream, ty: &syn::Type) -> TokenStream {
  let mut value = quote! { self.#field_name };
  if option_type_extract(ty).is_none() {
    let err = format!("Required child `{}` not specify", quote! { #ty });
    value.extend(quote! { .expect(#err)});
  };
  value
}
