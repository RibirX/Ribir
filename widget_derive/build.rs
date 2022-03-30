use std::env;
use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use syn::punctuated::Punctuated;
use syn::Ident;
use syn::Token;
use syn::{parse::Parse, LitStr};

struct BuiltinField {
  comment: LitStr,
  field_name: Ident,
  _colon_token: Token!(:),
  type_comment: LitStr,
}

impl Parse for BuiltinField {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    input.parse::<Token![#]>()?;
    let content;
    syn::bracketed!(content in input);
    content.parse::<Ident>()?;
    content.parse::<Token![=]>()?;

    Ok(BuiltinField {
      comment: content.parse()?,
      field_name: input.parse()?,
      _colon_token: input.parse()?,
      type_comment: input.parse()?,
    })
  }
}

struct BuiltinFields {
  fields: Punctuated<BuiltinField, Token![,]>,
}

mod kw {
  syn::custom_keyword!(doc);
}

impl Parse for BuiltinFields {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    input.parse::<Ident>()?;
    input.parse::<Token![!]>()?;
    let content;
    syn::braced!(content in input);

    let mut fields = Punctuated::new();

    while !content.is_empty() {
      if !content.peek2(Ident) {
        fields.push(content.parse()?);
        if content.is_empty() {
          break;
        }
        fields.push_punct(content.parse()?);
      } else {
        content.parse::<Token![#]>()?;
        content.parse::<Ident>()?;
      }
    }

    Ok(BuiltinFields { fields })
  }
}

impl Display for BuiltinFields {
  fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.fields.iter().try_for_each(|f| {
      write!(
        fmt,
        "- {} : {} \n \t - {}\n",
        &f.field_name,
        &f.type_comment.value(),
        &f.comment.value()
      )
    })
  }
}

fn main() {
  let tokens = proc_macro2::TokenStream::from_str(include_str!(
    "./src/widget_attr_macro/declare_widget/sugar_fields_struct.rs"
  ))
  .unwrap();

  let fields: BuiltinFields = syn::parse2(tokens).unwrap();

  let docs = format!("# Full builtin fields list \n\n{}", &fields);
  let out_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("../docs/declare_builtin_fields.md");
  fs::write(&dest_path, docs.as_str()).unwrap();
  println!("cargo:rerun-if-changed=build.rs");
}
