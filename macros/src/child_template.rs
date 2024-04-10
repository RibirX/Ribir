use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
  parse_quote, punctuated::Pair, spanned::Spanned, token::Comma, AngleBracketedGenericArguments,
  DataEnum, Field, Fields, FieldsNamed, FieldsUnnamed, GenericArgument, Index, PathArguments,
  PathSegment,
};

const BUILDER: &str = "Builder";
const ASSOCIATED_TEMPLATE: &str = "AssociatedTemplate";

pub(crate) fn derive_child_template(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: name, generics, data, .. } = input;
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let builder = Ident::new(&format!("{name}{BUILDER}"), name.span());

  let mut tokens = quote! {};

  let with_child_impl = |f_idx: usize, f: &mut Field, tokens: &mut TokenStream| {
    let field_name = if let Some(name) = f.ident.as_ref() {
      quote! {#name}
    } else {
      let f_idx = Index::from(f_idx);
      quote!(#f_idx)
    };
    let ty = option_type_extract(&f.ty).unwrap_or(&f.ty);

    let mut fill_gen = generics.clone();
    fill_gen.params.push(parse_quote!(_M));
    fill_gen.params.push(parse_quote!(_C));
    fill_gen
      .where_clause
      .get_or_insert_with(|| parse_quote! { where })
      .predicates
      .push(parse_quote!(#ty: ChildFrom<_C, _M>));

    let (g_impl, _, g_where) = fill_gen.split_for_impl();
    tokens.extend(quote! {
      impl #g_impl ComposeWithChild<_C, [_M; #f_idx]> for #builder #g_ty #g_where  {
        type Target = Self;
        #[track_caller]
        fn with_child(mut self, c: _C, ctx: &BuildCtx) -> Self::Target {
          assert!(self.#field_name.is_none(), "Try to fill same type twice.");
          self.#field_name = Some(ChildFrom::child_from(c, ctx));
          self
        }
      }
    });
  };
  match data {
    syn::Data::Struct(stt) => {
      tokens.extend(quote! {
        impl #g_impl Template for #name #g_ty #g_where {
          type Builder = #builder #g_ty;

          #[inline]
          fn builder() -> Self::Builder {  <_>::default() }
        }

        impl #g_impl Declare for #name #g_ty #g_where {
          type Builder = #builder #g_ty;
          #[inline]
          fn declarer() -> Self::Builder { #name::builder() }
        }

        impl #g_impl ObjDeclarer for #builder #g_ty {
          type Target = Self;
          #[inline]
          fn finish(self, _: &BuildCtx) -> Self { self }
        }


        impl #g_impl FromAnother<#builder #g_ty, ()> for #name #g_ty #g_where {
          #[inline]
          #[track_caller]
          fn from_another(value: #builder #g_ty, _: &BuildCtx) -> Self { value.build_tml() }
        }

        impl #g_impl std::convert::From<#builder #g_ty> for #name #g_ty #g_where {
          #[inline]
          #[track_caller]
          fn from(value: #builder #g_ty) -> Self { value.build_tml() }
        }

        impl #g_impl std::convert::From<#builder #g_ty> for Option<#name #g_ty> #g_where {
          #[inline]
          #[track_caller]
          fn from(value: #builder #g_ty) -> Self { Some(value.build_tml()) }
        }
      });

      match &mut stt.fields {
        Fields::Named(FieldsNamed { named: fields, .. }) => {
          fields
            .iter_mut()
            .enumerate()
            .for_each(|(f_idx, f)| with_child_impl(f_idx, f, &mut tokens));
          let builder_fields = fields
            .clone()
            .into_pairs()
            .map(convert_to_builder_pair);
          tokens.extend(quote! {
            #[derive(Default)]
            #vis struct #builder #g_impl #g_where {
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
            impl #g_impl TemplateBuilder for #builder #g_ty #g_where {
              type Target = #name #g_ty;
              #[inline]
              #[track_caller]
              fn build_tml(self) -> Self::Target {#name { #(#init_values),* }}
            }
          });
        }
        Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => {
          fields
            .iter_mut()
            .enumerate()
            .for_each(|(f_idx, f)| with_child_impl(f_idx, f, &mut tokens));
          let builder_fields = fields
            .clone()
            .into_pairs()
            .map(convert_to_builder_pair);
          tokens.extend(quote! {
            #[derive(Default)]
            #vis struct #builder #g_impl #g_where(#(#builder_fields)*);
          });

          let init_values = fields.iter().enumerate().map(|(idx, field)| {
            let idx = Index::from(idx);
            gen_init_value_tokens(quote!(#idx), &field.ty)
          });

          tokens.extend(quote! {
            impl #g_impl TemplateBuilder for #builder #g_ty #g_where {
              type Target = #name #g_ty;
              #[track_caller]
              fn build_tml(self) -> Self::Target {#name(#(#init_values),* ) }
            }
          });
        }
        Fields::Unit => {
          let err_str = format!("Can't derive `{ASSOCIATED_TEMPLATE}` for a empty template.",);
          return Err(syn::Error::new(Span::call_site(), err_str));
        }
      }
    }
    syn::Data::Enum(DataEnum { variants, .. }) => {
      let err_str = format!("Child `{}` not specify.", quote! { #name });
      tokens.extend(quote! {
        #[derive(Default)]
        #vis struct #builder #g_impl #g_where(Option<#name #g_ty>);

        impl #g_impl TemplateBuilder for #builder #g_ty #g_where {
          type Target = #name #g_ty;
          #[track_caller]
          fn build_tml(self) -> Self::Target {
            self.0.expect(&#err_str)
          }
        }
      });

      variants.iter().enumerate().for_each(|(idx, v)| {
        if let Fields::Unnamed(FieldsUnnamed { unnamed, .. }) = &v.fields {
          // only the enum variant has a single type need to implement fill convert.
          if unnamed.len() == 1 {
            let f = unnamed.first().unwrap();
            let ty = &f.ty;
            let v_name = &v.ident;
            let mut fill_gen = generics.clone();
            fill_gen.params.push(parse_quote!(_M));
            fill_gen.params.push(parse_quote!(_V));
            fill_gen
              .where_clause
              .get_or_insert_with(|| parse_quote! { where})
              .predicates
              .push(parse_quote!(#ty: ChildFrom<_V, _M>));

            let (g_impl, _, g_where) = fill_gen.split_for_impl();
            let idx = Index::from(idx);
            tokens.extend(quote! {
              impl #g_impl FromAnother<_V, [_M; #idx]> for #name #g_ty #g_where  {
                fn from_another(value: _V,  ctx: &BuildCtx) -> Self {
                  #name::#v_name(ChildFrom::child_from(value, ctx))
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

  let syn::Type::Path(ref path) = ty else {
    return None;
  };
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
