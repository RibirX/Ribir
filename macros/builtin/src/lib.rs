use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
  braced, bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, token, Ident,
  MetaNameValue, Path, Signature,
};

mod kw {
  use syn::custom_keyword;
  custom_keyword!(by);
}

enum Item {
  Field {
    doc_attr: MetaNameValue,
    mem: Ident,
    _colon: token::Colon,
    ty: syn::Type,
  },
  Method {
    doc_attr: MetaNameValue,
    sign: Signature,
  },
}
struct BuiltinWidget {
  ty: Path,
  _brace_token: token::Brace,
  items: Punctuated<Item, token::Comma>,
}
struct BuiltinWidgets {
  pub widgets: Vec<BuiltinWidget>,
}

impl Parse for Item {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    input.parse::<token::Pound>()?;
    let content;
    bracketed!(content in input);
    let doc_attr = content.parse()?;
    if input.peek(token::Fn) {
      Ok(Item::Method { doc_attr, sign: input.parse()? })
    } else {
      Ok(Item::Field {
        doc_attr,
        mem: input.parse()?,
        _colon: input.parse()?,
        ty: input.parse()?,
      })
    }
  }
}

impl Parse for BuiltinWidget {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    Ok(BuiltinWidget {
      ty: input.parse()?,
      _brace_token: braced!(content in input),
      items: Punctuated::parse_terminated(&content)?,
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

impl ToTokens for Item {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    match self {
      Item::Field { doc_attr, mem, _colon, ty } => {
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
      Item::Method { doc_attr, sign } => {
        let name = sign.ident.to_string();
        let sign = sign.to_token_stream().to_string();
        let doc = match &doc_attr.lit {
          syn::Lit::Str(str) => str,
          _ => unreachable!(),
        };
        tokens.extend(quote! {
          BuiltinMethod {
            name: #name,
            sign: #sign,
            doc: #doc,
          }
        })
      }
    }
  }
}

impl ToTokens for BuiltinWidget {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { ty, _brace_token, items } = self;
    let ty = ty.to_token_stream().to_string();
    let (fields, methods): (Vec<_>, Vec<_>) = items
      .iter()
      .partition(|item| matches!(item, Item::Field { .. }));

    tokens.extend(quote! {
      BuiltinWidget {
        ty: #ty,
        fields: &[#(#fields),*],
        methods:&[#(#methods),*]
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
        pub methods: &'static [BuiltinMethod],
      }

      pub struct BuiltinField {
        pub name: &'static str,
        pub ty: &'static str,
        pub doc: &'static str,
      }

      pub struct BuiltinMethod {
        pub name: &'static str,
        pub sign: &'static str,
        pub doc: &'static str,
      }

      pub static WIDGETS: [BuiltinWidget; #widget_size ] = [ #(#widgets), *];
    });
  }
}

#[proc_macro]
pub fn builtin(input: TokenStream) -> TokenStream {
  let widgets = parse_macro_input!(input as BuiltinWidgets);

  quote! { #widgets }.into()
}
