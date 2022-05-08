use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
  braced, bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, token, Ident,
  MetaNameValue, Type,
};

mod kw {
  use syn::custom_keyword;

  custom_keyword!(by);
}

struct Field {
  doc_attr: MetaNameValue,
  mem: Ident,
  _colon: token::Colon,
  ty: syn::Type,
}
struct BuiltinWidget {
  ty: Type,
  _brace_token: token::Brace,
  fields: Punctuated<Field, token::Comma>,
}
struct BuiltinWidgets {
  pub widgets: Vec<BuiltinWidget>,
}

impl Parse for Field {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    input.parse::<token::Pound>()?;
    let doc;
    bracketed!(doc in input);
    Ok(Field {
      doc_attr: doc.parse()?,
      mem: input.parse()?,
      _colon: input.parse()?,
      ty: input.parse()?,
    })
  }
}

impl Parse for BuiltinWidget {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;

    Ok(BuiltinWidget {
      ty: input.parse()?,
      _brace_token: braced!(content in input),
      fields: Punctuated::parse_terminated(&content)?,
    })
  }
}

impl Parse for BuiltinWidgets {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut widgets = vec![];
    loop {
      if input.is_empty() {
        break;
      }
      widgets.push(input.parse()?);
    }
    Ok(BuiltinWidgets { widgets })
  }
}

impl ToTokens for Field {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { doc_attr, mem, ty, .. } = self;
    let ty = quote! { #ty }.to_string();
    let name = mem.to_string();
    let doc = match &doc_attr.lit {
      syn::Lit::Str(str) => str,
      _ => unreachable!(),
    };
    tokens.extend(quote! {
      BuiltinField {
        name: #name,
        ty: #ty,
        doc: #doc,
      }
    })
  }
}

impl ToTokens for BuiltinWidget {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let ty = self.ty.to_token_stream().to_string();
    let fields = &self.fields;
    tokens.extend(quote! {
      BuiltinWidget {
        ty: #ty,
        fields: &[#fields]
      }
    });
  }
}

impl ToTokens for BuiltinWidgets {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let widgets = &self.widgets;
    let widget_size = self.widgets.len();
    tokens.extend(quote! {
      pub struct BuiltinWidget {
        pub ty: &'static str,
        pub fields: &'static [BuiltinField],
      }

      pub struct BuiltinField {
        pub name: &'static str,
        pub ty: &'static str,
        pub doc: &'static str,
      }

      pub static WIDGETS: [BuiltinWidget; #widget_size ] = [ #(#widgets), *];
    })
  }
}

#[proc_macro]
pub fn builtin(input: TokenStream) -> TokenStream {
  let widgets = parse_macro_input!(input as BuiltinWidgets);

  quote! { #widgets }.into()
}
