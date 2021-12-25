use crate::util::{struct_unwrap, suffix_ident};
use proc_macro::{Diagnostic, Level};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Fields, Meta};

pub const DECLARE: &str = "Declare";
pub const BUILDER: &str = "Builder";
pub const RENAME: &str = "rename";

pub(crate) fn declare_derive(input: &mut syn::DeriveInput) -> Result<TokenStream, TokenStream> {
  let vis = &input.vis;
  let name = &input.ident;
  let mut g_default = input.generics.clone();
  let (g_impl, g_ty, g_where) = input.generics.split_for_impl();

  let stt = struct_unwrap(&mut input.data, DECLARE)?;
  let fields = match &stt.fields {
    Fields::Named(named) => named.named.clone(),
    Fields::Unit => <_>::default(),
    Fields::Unnamed(unnamed) => {
      let err = format!("`{}` not be supported to derive for tuple struct", DECLARE);
      return Err(quote_spanned! { unnamed.span() => compile_error!(#err)});
    }
  };

  let builder = suffix_ident(BUILDER, &quote! {#name});

  let mut builder_fields = fields.clone();
  builder_fields.iter_mut().try_for_each(|f| {
    if let Some(idx) = f
      .attrs
      .iter_mut()
      .position(|attr| attr.path.is_ident(RENAME))
    {
      let attr = f.attrs.remove(idx);
      let meta = match attr.parse_meta() {
        Ok(meta) => meta,
        Err(err) => return Err(err.into_compile_error()),
      };
      if let Meta::NameValue(nv) = meta {
        if let syn::Lit::Str(ref new_name) = nv.lit {
          f.ident = Some(syn::Ident::new(new_name.value().as_str(), nv.span()));
        } else {
          return Err(quote_spanned! { nv.span() => compile_error!("Invalid rename meta.") });
        }
      }
    }
    Ok(())
  })?;

  let fields_ident = fields.iter().map(|f| f.ident.as_ref().unwrap());
  let c_fields_ident = fields_ident.clone();

  let builder_fields_ident = builder_fields.iter().map(|f| f.ident.as_ref().unwrap());
  let c_builder_fields_ident = builder_fields_ident.clone();

  let reserve_ident = &crate::declare_func_derive::sugar_fields::RESERVE_IDENT;
  builder_fields.iter().for_each(|f| {
    f.ident.as_ref().and_then(|name| {
      reserve_ident.get(name.to_string().as_str()).map(|doc| {
        let msg = format!("the identify `{}` is reserved to {}", name, &doc);
        Diagnostic::spanned(vec![name.span().unwrap()], Level::Error, msg)
          .help(format! {
            "rename it in builder to ignore the conflict \n\
            ```\n\
            #[rename = \"xxx\"] \n\
            {}\n\
            ```
            ", quote!{ #f }
          })
          .emit();
      })
    });
  });

  crate::util::add_where_bounds(
    &mut g_default,
    quote! {#name:
    Default},
  );
  let (g_d_impl, g_d_ty, g_d_where) = g_default.split_for_impl();

  let tokens = quote! {
    #vis struct #builder #g_ty #g_where {
      #builder_fields
    }

    impl #g_impl Declare for #name #g_ty #g_where {
      type Builder = #builder #g_ty;
    }

    impl #g_impl DeclareBuilder for #builder #g_ty #g_where {
      type Target = #name #g_ty;
      #[inline]
      fn build(self) -> Self::Target {
        #name { #(#fields_ident : self.#builder_fields_ident),* }
      }
    }

    impl #g_d_impl Default for #builder #g_d_ty #g_d_where {
      #[inline]
      fn default() -> Self {
        let temp = #name::default();
        #builder { #(#c_builder_fields_ident : temp.#c_fields_ident),*}
      }
    }
  };

  Ok(tokens)
}
