use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
  AngleBracketedGenericArguments, DataEnum, Field, Fields, FieldsNamed, FieldsUnnamed,
  GenericArgument, Index, Meta, PathArguments, PathSegment, Type, Visibility, parse_quote,
  spanned::Spanned,
};

const BUILDER: &str = "Builder";
const TEMPLATE: &str = "Template";
fn with_child_generics(generics: &syn::Generics, child_ty: &Type) -> syn::Generics {
  let mut gen = generics.clone();
  gen.params.push(parse_quote!('_c));
  gen.params.push(parse_quote!(_C));
  gen.params.push(parse_quote!(const _M: usize));

  let predicates = &mut gen
    .where_clause
    .get_or_insert_with(|| parse_quote! { where })
    .predicates;
  predicates.push(parse_quote!(_C: IntoChildCompose<#child_ty, _M>));

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

    impl #g_impl ChildOfCompose for #name #g_ty {}

    impl #g_impl ComposeChildFrom<#builder #g_ty, 1> for #name #g_ty {
      #[inline]
      fn compose_child_from(from: #builder #g_ty) -> Self { from.build_tml() }
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

      let with_child_impl = |f_idx: usize, f: &Field, tokens: &mut TokenStream| {
        let field_name = if let Some(name) = f.ident.as_ref() {
          quote! {#name}
        } else {
          let f_idx = Index::from(f_idx);
          quote!(#f_idx)
        };
        let ty = option_type_extract(&f.ty).unwrap_or(&f.ty);

        let gen = with_child_generics(generics, ty);
        let (g_impl, _, g_where) = gen.split_for_impl();
        tokens.extend(quote! {
          impl #g_impl ComposeWithChild<'_c, _C, false, 1, {#f_idx + 1}, _M>
            for #builder #g_ty #g_where
          {
            type Target = Self;
            #[track_caller]
            fn with_child(mut self, c: _C) -> Self::Target {
              assert!(self.#field_name.is_none(), concat!("Already has a `", stringify!(#ty), "` child"));
              self.#field_name = Some(c.into_child_compose());
              self
            }
          }
        });
      };

      match &mut stt.fields {
        Fields::Named(FieldsNamed { named, .. }) => {
          let builder_fields_def = named.iter().map(move |f| {
            let ty = &f.ty;
            let name = f.ident.as_ref().unwrap();
            if option_type_extract(ty).is_none() {
              quote! { #name: Option<#ty> }
            } else {
              quote! { #name: #ty }
            }
          });
          let names = named.iter().map(|f| f.ident.as_ref().unwrap());
          tokens.extend(quote! {
            #vis struct #builder #g_impl #g_where {
              #(#builder_fields_def),*
            }

            impl #g_impl Default for #builder #g_ty #g_where {
              #[inline]
              fn default() -> Self { Self { #(#names: None),* } }
            }
          });

          let (mut declare_fields, mut field_value, mut child) = (vec![], vec![], vec![]);
          named.iter_mut().for_each(|f| {
            if let Some(field) = take_template_field(f) {
              declare_fields.push(&*f);
              field_value.push(field.value);
            } else {
              child.push(&*f);
            }
          });

          if !declare_fields.is_empty() {
            let declare_methods = declare_field_methods(vis, &declare_fields);
            tokens.extend(quote! {
              impl #g_impl #builder #g_ty #g_where {
                #(#declare_methods)*
              }
            });
          }

          child
            .iter()
            .enumerate()
            .for_each(|(f_idx, f)| with_child_impl(f_idx, f, &mut tokens));

          let init_values = declare_fields
            .iter()
            .zip(field_value.iter())
            .map(|(field, value)| gen_init_field_tokens(field, value))
            .chain(child.iter().map(|field| {
              let field_name = field.ident.as_ref().unwrap();
              let ty = &field.ty;
              let value = gen_init_child_tokens(quote!(#field_name), ty);
              quote! {#field_name: #value}
            }));

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
          let nones = fields.iter().map(|_| quote! { None });
          let builder_fields_def = fields.iter().map(|f| {
            let ty = &f.ty;
            if option_type_extract(ty).is_none() {
              quote! { Option<#ty> }
            } else {
              quote! { #ty }
            }
          });
          tokens.extend(quote! {
            #vis struct #builder #g_impl #g_where(#(#builder_fields_def),*);

            impl #g_impl Default for #builder #g_ty #g_where {
              #[inline]
              fn default() -> Self { Self(#(#nones),*) }
            }
          });

          let init_values = fields.iter().enumerate().map(|(idx, field)| {
            let idx = Index::from(idx);
            gen_init_child_tokens(quote!(#idx), &field.ty)
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
      };

      Ok(tokens)
    }
    syn::Data::Enum(DataEnum { variants, .. }) => {
      let err_str = format!("Child `{}` not specify.", quote! { #name });
      tokens.extend(quote! {
        #vis struct #builder #g_impl #g_where(Option<#name #g_ty>);

        impl #g_impl Default for #builder #g_ty #g_where {
          fn default() -> Self {
            Self(None)
          }
        }

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
            let gen = with_child_generics(generics, ty);
            let (g_impl, _, g_where) = gen.split_for_impl();
            tokens.extend(quote! {
              impl #g_impl ComposeWithChild<'_c, _C, false, 1, {#i + 1}, _M>
                for #builder #g_ty #g_where
              {
                type Target = Self;
                #[track_caller]
                fn with_child(mut self, c: _C) -> Self::Target {
                  assert!(self.0.is_none(), concat!("Already has a `", stringify!(#ty), "` child."));
                  self.0 = Some(#name::#v_name(c.into_child_compose()));
                  self
                }
              }
            });
          }
        }
      });
      Ok(tokens)
    }
    syn::Data::Union(u) => {
      let err_str = format!("`{TEMPLATE}` not support for Union");
      Err(syn::Error::new(u.union_token.span(), err_str))
    }
  }
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

fn gen_init_child_tokens(field_name: TokenStream, ty: &syn::Type) -> TokenStream {
  let mut value = quote! { self.#field_name };
  if option_type_extract(ty).is_none() {
    let err = format!("Required child `{}` not specify", quote! { #ty });
    value.extend(quote! { .expect(#err)});
  };
  value
}

fn gen_init_field_tokens(field: &Field, default: &Option<syn::Expr>) -> TokenStream {
  let field_name = field.ident.as_ref().unwrap();
  let ty = &field.ty;
  if let Some(df) = default {
    quote! { #field_name: self.#field_name.unwrap_or_else(|| #df.template_field_into()) }
  } else if option_type_extract(ty).is_none() {
    let err = format!("Required field `{}: {}` not set", field_name, ty.to_token_stream());
    quote! { #field_name: self.#field_name.expect(#err) }
  } else {
    quote! { #field_name: self.#field_name }
  }
}

struct TemplateField {
  value: Option<syn::Expr>,
}

fn take_template_field(field: &mut syn::Field) -> Option<TemplateField> {
  let pos = field
    .attrs
    .iter()
    .position(|attr| attr.path().is_ident("template"))?;

  let attr = field.attrs.remove(pos);
  let meta = attr
    .parse_args::<Meta>()
    .expect("Unsupported meta!");
  if !meta.path().is_ident("field") {
    panic!("template only support `field` now");
  }

  match meta {
    Meta::Path(_) => Some(TemplateField { value: None }),
    Meta::List(_) => panic!("`field` not support list value"),
    Meta::NameValue(nv) => Some(TemplateField { value: Some(nv.value) }),
  }
}

fn declare_field_methods<'a>(
  vis: &'a Visibility, fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
  fields.iter().map(move |field| {
    let field_name = field.ident.as_ref().unwrap();
    let doc = crate::util::doc_attr(field);
    let ty = &field.ty;

    quote! {
      #[inline]
      #[allow(clippy::type_complexity)]
      #doc
      #vis fn #field_name<const _M: usize>(&mut self, v: impl TemplateFieldInto<#ty, _M>)
        -> &mut Self
      {
        self.#field_name = Some(v.template_field_into());
        self
      }
    }
  })
}
