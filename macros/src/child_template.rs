use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
  AngleBracketedGenericArguments, DataEnum, Field, Fields, FieldsNamed, FieldsUnnamed,
  GenericArgument, Index, PathArguments, PathSegment, Type, parse_quote, punctuated::Pair,
  spanned::Spanned, token::Comma,
};

const BUILDER: &str = "Builder";
const TEMPLATE: &str = "Template";
fn with_child_generics(generics: &syn::Generics, child_ty: &Type, m: usize) -> syn::Generics {
  let mut gen = generics.clone();
  gen.params.push(parse_quote!('_c));
  gen.params.push(parse_quote!(_C));

  let predicates = &mut gen
    .where_clause
    .get_or_insert_with(|| parse_quote! { where })
    .predicates;
  predicates.push(parse_quote!(_C: IntoChild<#child_ty, #m>));
  predicates.push(parse_quote!(#child_ty: '_c));
  predicates.push(parse_quote!(Self: '_c));
  gen
}

pub(crate) fn derive_child_template(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: name, generics, data, .. } = input;
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let builder = Ident::new(&format!("{name}{BUILDER}"), name.span());

  let mut tokens = quote! {
    impl #g_impl Template for #name #g_ty #g_where {
      type Builder = #builder #g_ty;

      #[inline]
      fn builder() -> Self::Builder {  <_>::default() }
    }

    impl #g_impl IntoChild<#name #g_ty, 0> for #builder #g_ty {
      #[inline]
      fn into_child(self) -> #name #g_ty { self.build_tml()  }
    }

    impl #g_impl IntoChild<Option<#name #g_ty>, 0> for #builder #g_ty {
      #[inline]
      fn into_child(self) -> Option<#name #g_ty> { Some(self.build_tml())  }
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
  };

  let with_child_impl = |f_idx: usize, f: &mut Field, tokens: &mut TokenStream| {
    let field_name = if let Some(name) = f.ident.as_ref() {
      quote! {#name}
    } else {
      let f_idx = Index::from(f_idx);
      quote!(#f_idx)
    };
    let ty = option_type_extract(&f.ty).unwrap_or(&f.ty);

    for m in 0..4 {
      let with_m = 4 * f_idx + m;
      let gen = with_child_generics(generics, ty, m);
      let (g_impl, _, g_where) = gen.split_for_impl();
      tokens.extend(quote! {
        impl #g_impl WithChild<'_c, _C, 2, #with_m> for #builder #g_ty #g_where  {
          type Target = Self;
          #[track_caller]
          fn with_child(mut self, c: _C) -> Self::Target {
            assert!(self.#field_name.is_none(), "Try to fill same type twice.");
            self.#field_name = Some(c.into_child());
            self
          }
        }
      });
    }
  };
  match data {
    syn::Data::Struct(stt) => {
      tokens.extend(quote! {
        impl #g_impl Declare for #name #g_ty #g_where {
          type Builder = #builder #g_ty;
          #[inline]
          fn declarer() -> Self::Builder { #name::builder() }
        }

        impl #g_impl ObjDeclarer for #builder #g_ty {
          type Target = Self;
          #[inline]
          fn finish(self) -> Self { self }
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
          let err_str = format!("Can't derive `{TEMPLATE}` for a empty template.",);
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

      variants.iter().enumerate().for_each(|(i, v)| {
        if let Fields::Unnamed(FieldsUnnamed { unnamed, .. }) = &v.fields {
          // only the enum variant has a single type need to implement fill convert.
          if unnamed.len() == 1 {
            let f = unnamed.first().unwrap();
            let ty = &f.ty;
            let v_name = &v.ident;
            for m in 0..4 {
              let with_m = 4 * i + m;
              let gen = with_child_generics(generics, ty, m);
              let (g_impl, _, g_where) = gen.split_for_impl();
              tokens.extend(quote! {
                impl #g_impl WithChild<'_c, _C, 2, #with_m> for #builder #g_ty #g_where
                {
                  type Target = Self;
                  #[track_caller]
                  fn with_child(mut self, c: _C) -> Self::Target {
                    assert!(self.0.is_none(), "Try to fill same type twice.");
                    self.0 = Some(#name::#v_name(c.into_child()));
                    self
                  }
                }
              });
            }
          }
        }
      });
    }
    syn::Data::Union(u) => {
      let err_str = format!("`{TEMPLATE}` not support for Union");
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
      iter.next().is_none_or(|s| {
        match_ident(s, "option")
          && iter
            .next()
            .is_none_or(|s| match_ident(s, "std") || match_ident(s, "core"))
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
