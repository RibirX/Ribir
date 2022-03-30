use crate::util::struct_unwrap;
use proc_macro::{Diagnostic, Level};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
  parse::Parse, parse_quote, punctuated::Punctuated, spanned::Spanned, Fields, Ident, Result,
};

const DECLARE: &str = "Declare";
const BUILDER: &str = "Builder";
const DECLARE_ATTR: &str = "declare";

struct DefaultValue {
  _default_token: kw::default,
  _eq_token: Option<syn::token::Eq>,
  value: Option<syn::LitStr>,
}
#[derive(Default)]
struct FieldBuilderAttr {
  rename: Option<syn::LitStr>,
  builtin: Option<kw::builtin>,
  strip_option: Option<kw::strip_option>,
  default: Option<DefaultValue>,
}

mod kw {
  use syn::custom_keyword;
  custom_keyword!(rename);
  custom_keyword!(strip_option);
  custom_keyword!(builtin);
  custom_keyword!(default);
}

pub fn field_convert_method(field_name: &Ident) -> Ident {
  Ident::new(&format!("{}{}", "into_", field_name), field_name.span())
}
impl Parse for DefaultValue {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    Ok(Self {
      _default_token: input.parse()?,
      _eq_token: input.parse()?,
      value: input.parse()?,
    })
  }
}
impl Parse for FieldBuilderAttr {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let mut attr = FieldBuilderAttr::default();
    while !input.is_empty() {
      let lookahead = input.lookahead1();

      // use input instead of lookahead to peek builtin, because need't complicate in
      // compile error.
      if input.peek(kw::builtin) {
        attr.builtin = Some(input.parse()?);
      } else if lookahead.peek(kw::rename) {
        input.parse::<kw::rename>()?;
        input.parse::<syn::Token![=]>()?;
        attr.rename = Some(input.parse()?);
      } else if lookahead.peek(kw::strip_option) {
        attr.strip_option = Some(input.parse()?);
      } else if lookahead.peek(kw::default) {
        attr.default = Some(input.parse()?);
      } else {
        return Err(lookahead.error());
      }
      if let (Some(rename), Some(builtin)) = (attr.rename.as_ref(), attr.builtin.as_ref()) {
        let mut d = Diagnostic::new(
          Level::Error,
          "`rename` and `builtin` can not be used in same time.",
        );
        d.set_spans(vec![rename.span().unwrap(), builtin.span().unwrap()]);
        d.emit();
      }
      if !input.is_empty() {
        input.parse::<syn::Token![,]>()?;
      }
    }
    Ok(attr)
  }
}

pub(crate) fn declare_derive(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let vis = &input.vis;
  let name = &input.ident;

  let (g_impl, g_ty, g_where) = input.generics.split_for_impl();

  let stt = struct_unwrap(&mut input.data, DECLARE)?;

  let mut builder_fields = Punctuated::default();
  match &mut stt.fields {
    Fields::Named(named) => {
      named
        .named
        .pairs_mut()
        .try_for_each::<_, syn::Result<()>>(|mut pair| {
          let idx = pair
            .value()
            .attrs
            .iter()
            .position(|attr| attr.path.is_ident(DECLARE_ATTR));
          let builder_attr = if let Some(idx) = idx {
            let attr = pair.value_mut().attrs.remove(idx);
            let args: FieldBuilderAttr = attr.parse_args()?;
            Some(args)
          } else {
            None
          };

          builder_fields.push(((*pair.value()).clone(), builder_attr));
          if let Some(c) = pair.punct() {
            builder_fields.push_punct(**c);
          }

          Ok(())
        })?;
    }
    Fields::Unit => <_>::default(),
    Fields::Unnamed(unnamed) => {
      let err = syn::Error::new(
        unnamed.span(),
        format!("`{}` not be supported to derive for tuple struct", DECLARE),
      );
      return Err(err);
    }
  };

  let builder = Ident::new(&format!("{}{}", name, BUILDER), name.span());

  // rename fields if need
  builder_fields
    .iter_mut()
    .try_for_each::<_, syn::Result<()>>(|(f, attr)| {
      if let Some(new_name) = attr.as_ref().and_then(|attr| attr.rename.as_ref()) {
        f.ident = Some(syn::Ident::new(new_name.value().as_str(), new_name.span()));
      }
      Ok(())
    })?;

  // reverse name check.
  let reserve_ident = &crate::widget_attr_macro::RESERVE_IDENT;
  builder_fields
    .iter_mut()
    .filter_map(|(f, attr)| {
      let not_builtin = attr.as_ref().map_or(true, |attr| attr.builtin.is_none());
      not_builtin.then(|| f)
    })
    .for_each(|f| {
      let field_name = f.ident.as_ref().unwrap();
      if let Some(doc) = reserve_ident.get(field_name.to_string().as_str()) {
        let msg = format!("the identify `{}` is reserved to {}", field_name, &doc);
        // not display the attrs in the help code.
        f.attrs.clear();
        Diagnostic::spanned(vec![field_name.span().unwrap()], Level::Error, msg)
          .help(format! {
            "use `rename` meta to avoid the name conflict in `widget!` macro.\n\n\
            #[declare(rename = \"xxx\")] \n\
            {}", quote!{ #f }
          })
          .emit();
      }
    });

  // builder define
  let def_fields = builder_fields.pairs().map(|p| {
    let ((f, _), c) = p.into_tuple();
    let mut f = f.clone();
    let ty = &f.ty;
    f.ty = parse_quote!(Option<#ty>);
    syn::punctuated::Pair::new(f, c)
  });
  let mut tokens = quote! {
    #vis struct #builder #g_ty #g_where {
      #(#def_fields)*
    }
  };

  let mut methods = quote! {};
  builder_fields
    .iter()
    .try_for_each::<_, syn::Result<()>>(|(f, attr)| {
      let name = f.ident.as_ref().unwrap();
      let fn_convert = field_convert_method(name);

      let ty = &f.ty;
      let methods_tokens =
        if let Some(FieldBuilderAttr { strip_option: Some(strip_option), .. }) = attr {
          let strip_ty = extract_type_from_option(ty).ok_or_else(|| {
            syn::Error::new(
              strip_option.span(),
              "Can't use meta `strip_option` for a non Option type ",
            )
          })?;
          quote! {
            #[inline]
            #[allow(non_snake_case)]
            #vis fn #fn_convert<M, V: Striped<M, #strip_ty>>(v: V) -> #ty
            {
               v.striped()
            }

            #[inline]
            #vis fn #name<M, V: Striped<M, #strip_ty>>(mut self, v: V) -> Self
            {
              self.#name = Some(v.striped());
              self
            }
          }
        } else {
          quote! {
            #[inline]
            #[allow(non_snake_case)]
            #vis fn #fn_convert<V: Into<#ty>>(v: V) -> #ty { v.into() }

            #[inline]
            #vis fn #name<V: Into<#ty>>(mut self, v: V) -> Self {
              self.#name = Some(Self::#fn_convert(v));
              self
            }
          }
        };

      methods.extend(quote! {
        #methods_tokens
      });

      Ok(())
    })?;

  // implement declare trait
  let fields_ident = stt.fields.iter().map(|f| f.ident.as_ref());

  let builder_fields_ident = builder_fields
    .iter()
    .map(|(f, _)| f.ident.as_ref().unwrap());

  let value = builder_fields.iter().map(|(f, attr)| {
    let field_name = f.ident.as_ref().unwrap();
    let or_default = attr.as_ref().and_then(|a| a.default.as_ref()).map(|d| {
      let expr = match &d.value {
        Some(v) => {
          let expr: syn::Expr = v.parse().unwrap();
          let fn_convert = field_convert_method(field_name);
          quote! {Self::#fn_convert(#expr)}
        }
        None => {
          quote! {<_>::default()}
        }
      };

      quote! {
        .or_else(|| { Some(#expr) })
      }
    });
    quote! {
      self.#field_name
      #or_default
      .expect(&format!("Required field `{}::{}` not set", stringify!(#name), stringify!(#field_name)))
    }
  });

  tokens.extend(quote! {
    impl #g_impl Declare for #name #g_ty #g_where {
      type Builder = #builder #g_ty;

      fn builder() -> Self::Builder {
        #builder { #(#builder_fields_ident : None ),*}
      }
    }

    impl #g_impl DeclareBuilder for #builder #g_ty #g_where {
      type Target = #name #g_ty;
      #[inline]
      #[allow(dead_code)]
      fn build(self, ctx: &mut BuildCtx) -> Self::Target {
        #name {
          #(#fields_ident : #value),* }
      }
    }
  });
  // field converter
  tokens.extend(quote! {
    impl #g_impl #builder #g_ty #g_where {
      #methods
    }
  });

  Ok(tokens)
}

// code from https://stackoverflow.com/questions/55271857/how-can-i-get-the-t-from-an-optiont-when-using-syn
fn extract_type_from_option(ty: &syn::Type) -> Option<&syn::Type> {
  use syn::{GenericArgument, Path, PathArguments, PathSegment};

  fn extract_type_path(ty: &syn::Type) -> Option<&Path> {
    match *ty {
      syn::Type::Path(ref typepath) if typepath.qself.is_none() => Some(&typepath.path),
      _ => None,
    }
  }

  fn extract_option_segment(path: &Path) -> Option<&PathSegment> {
    let idents_of_path = path
      .segments
      .iter()
      .into_iter()
      .fold(String::new(), |mut acc, v| {
        acc.push_str(&v.ident.to_string());
        acc.push('|');
        acc
      });
    vec!["Option|", "std|option|Option|", "core|option|Option|"]
      .into_iter()
      .find(|s| idents_of_path == *s)
      .and_then(|_| path.segments.last())
  }

  extract_type_path(ty)
    .and_then(extract_option_segment)
    .and_then(|path_seg| {
      let type_params = &path_seg.arguments;
      // It should have only on angle-bracketed param ("<String>"):
      match *type_params {
        PathArguments::AngleBracketed(ref params) => params.args.first(),
        _ => None,
      }
    })
    .and_then(|generic_arg| match *generic_arg {
      GenericArgument::Type(ref ty) => Some(ty),
      _ => None,
    })
}
