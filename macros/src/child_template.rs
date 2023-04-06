use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
  parse::Parse, parse_quote, punctuated::Pair, spanned::Spanned, token::Comma,
  AngleBracketedGenericArguments, DataEnum, Field, Fields, FieldsNamed, FieldsUnnamed,
  GenericArgument, Index, PathArguments, PathSegment,
};

const TML: &str = "Tml";
const ASSOCIATED_TEMPLATE: &str = "AssociatedTemplate";
const TEMPLATE_ATTR: &str = "template";

#[derive(Default)]
struct TemplateAttr {
  /// `flat_fill` let a template type support init by its template fill item.
  _flat_fill: kw::flat_fill,
}

mod kw {
  use syn::custom_keyword;
  custom_keyword!(flat_fill);
}

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

  let fill_tml_impl = |mut f_idx: usize, f: &mut Field, tokens: &mut TokenStream| {
    let field_name = if let Some(name) = f.ident.as_ref() {
      quote! {#name}
    } else {
      let f_idx = Index::from(f_idx);
      quote!(#f_idx)
    };
    let ty = option_type_extract(&f.ty).unwrap_or(&f.ty);

    let mut fill_gen = generics.clone();
    fill_gen.params.push(parse_quote!(_Marker: ImplMarker));
    fill_gen
      .params
      .push(parse_quote!(_Child: IntoChild<_Marker, #ty>));

    let (g_impl, _, g_where) = fill_gen.split_for_impl();
    tokens.extend(quote! {
      impl #g_impl FillTml<[_Marker; #f_idx], _Child> for #tml #g_ty #g_where  {
        fn fill_tml(&mut self, c: _Child) {
          assert!(self.#field_name.is_none(), "Try to fill same type twice.");
          self.#field_name = Some(c.into_child());
        }
      }
    });
    let idx = f
      .attrs
      .iter()
      .position(|attr| attr.path.is_ident(TEMPLATE_ATTR));
    let tml_attr = idx.map(|idx| f.attrs.remove(idx));
    if let Some(tml_attr) = tml_attr {
      let _: TemplateAttr = tml_attr
        .parse_args()
        .expect("Only #[template(flat_fill) support");
      let mut flat_fill_gen = generics.clone();
      flat_fill_gen.params.push(parse_quote!(_Child));
      flat_fill_gen.params.push(parse_quote!(_Marker));
      let predicates = &mut flat_fill_gen
        .where_clause
        .get_or_insert_with(|| parse_quote! { where})
        .predicates;
      predicates.push(parse_quote! { #ty: Template});
      predicates.push(parse_quote! {
        <#ty as Template>::Builder: TemplateBuilder<Target = #ty> + FillTml<_Marker, _Child>
      });

      f_idx += 655536;
      let (g_impl, _, g_where) = flat_fill_gen.split_for_impl();
      tokens.extend(quote! {
        impl #g_impl FillTml<[_Marker; #f_idx], _Child> for #tml #g_ty #g_where {
          fn fill_tml(&mut self, c: _Child) {
            let mut builder = <#ty>::builder();
            builder.fill_tml(c);
            self.fill_tml(builder.build_tml());
          }
        }
      });
    }
  };
  match data {
    syn::Data::Struct(stt) => match &mut stt.fields {
      Fields::Named(FieldsNamed { named: fields, .. }) => {
        fields
          .iter_mut()
          .enumerate()
          .for_each(|(f_idx, f)| fill_tml_impl(f_idx, f, &mut tokens));
        let builder_fields = fields.clone().into_pairs().map(convert_to_builder_pair);
        tokens.extend(quote! {
          #[derive(Default)]
          #vis struct #tml #g_impl #g_where {
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
      }
      Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => {
        fields
          .iter_mut()
          .enumerate()
          .for_each(|(f_idx, f)| fill_tml_impl(f_idx, f, &mut tokens));
        let builder_fields = fields.clone().into_pairs().map(convert_to_builder_pair);
        tokens.extend(quote! {
          #[derive(Default)]
          #vis struct #tml #g_impl #g_where(#(#builder_fields)*);
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
        #vis struct #tml #g_impl #g_where(Option<#name #g_ty>);

        impl #g_impl TemplateBuilder for #tml #g_ty #g_where {
          type Target = #name #g_ty;
          #[inline]
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
            let idx = Index::from(idx);

            fill_gen.params.push(parse_quote!(_Marker: ImplMarker));
            fill_gen
              .params
              .push(parse_quote!(_Child: IntoEnumVariable<_Marker, #ty>));

            let (g_impl, _, g_where) = fill_gen.split_for_impl();

            tokens.extend(quote! {
              impl #g_impl FillTml<[_Marker;#idx], _Child> for #tml #g_ty #g_where  {
                fn fill_tml(&mut self, c: _Child) {
                  assert!(self.0.is_none(), "Try to fill enum template with two variant.");
                  self.0 = Some(#name::#v_name(c.into_variable()));
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

impl Parse for TemplateAttr {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(Self { _flat_fill: input.parse()? })
  }
}
