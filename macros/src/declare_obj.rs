use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Comma},
  Ident, Macro, Path,
};

use crate::{
  error::Error,
  rdl_macro::{DeclareField, RdlParent, StructLiteral},
  variable_names::BUILTIN_INFOS,
};

pub struct DeclareObj<'a> {
  span: Span,
  this: ObjNode<'a>,
  children: &'a Vec<Macro>,
}
enum ObjType<'a> {
  Type { span: Span, ty: &'a Path },
  Var(&'a Ident),
}
struct ObjNode<'a> {
  node_type: ObjType<'a>,
  fields: &'a Punctuated<DeclareField, Comma>,
}

impl<'a> DeclareObj<'a> {
  pub fn from_literal(mac: &'a StructLiteral) -> Result<Self, TokenStream> {
    let StructLiteral { span, parent, fields, children } = mac;
    let span = *span;
    let node_type = match parent {
      RdlParent::Type(ty) => ObjType::Type { ty, span },
      RdlParent::Var(name) => ObjType::Var(name),
    };
    let this = ObjNode { node_type, fields };
    Ok(Self { this, span, children })
  }
}

impl<'a> ToTokens for DeclareObj<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { this, span, children } = self;
    if children.is_empty() {
      self.gen_node_tokens(tokens);
    } else {
      Brace(*span).surround(tokens, |tokens| {
        let mut node = quote! {};
        self.gen_node_tokens(&mut node);
        match &self.this.node_type {
          ObjType::Type { span, .. } => {
            // declare the host widget before  children. So that we can use variable if it
            // moved in children. This is consistent with the user's declaration.
            let name = Ident::new("_ribir_ಠ_ಠ", self.span);
            quote_spanned! { *span => let _ribir_ಠ_ಠ = #node; }.to_tokens(tokens);
            self.declare_children_and_compose_it(&name, tokens);
          }
          ObjType::Var(var) => {
            if !this.fields.is_empty() {
              quote_spanned! { var.span() => let #var = #node; }.to_tokens(tokens);
            }
            self.declare_children_and_compose_it(var, tokens)
          }
        };
      })
    }
  }
}

impl<'a> DeclareObj<'a> {
  pub fn error_check(&self) -> Result<(), Error> {
    if let ObjType::Var(_) = self.this.node_type {
      let invalid_fields = self
        .this
        .fields
        .iter()
        .filter(|f| !BUILTIN_INFOS.contains_key(&f.member.to_string()))
        .collect::<Vec<_>>();
      if !invalid_fields.is_empty() {
        return Err(Error::InvalidFieldInVar(invalid_fields.into()));
      }
    }

    Ok(())
  }

  fn declare_children_and_compose_it(&self, var: &Ident, tokens: &mut TokenStream) {
    let mut children = vec![];
    for (i, c) in self.children.iter().enumerate() {
      let child = Ident::new(&format!("_child_{i}_ಠ_ಠ"), c.span());
      quote_spanned! { c.span() => let #child = #c; }.to_tokens(tokens);
      children.push(child)
    }

    quote_spanned! { self.span => #var #(.with_child(#children, ctx!()))* }.to_tokens(tokens)
  }

  fn gen_node_tokens(&self, tokens: &mut TokenStream) {
    let ObjNode { node_type, fields } = &self.this;
    match node_type {
      ObjType::Type { ty, span } => {
        if fields.is_empty() {
          quote_spanned! { *span => #ty::declarer().finish(ctx!())}.to_tokens(tokens);
        } else {
          let fields = fields.iter();
          // we not gen chain call to avoid borrow twice. e.g.
          // ```
          // let mut x = ...;
          // X::declarer().a(&mut x.a).b(&mut x.b).finish(ctx!());
          // ```
          // `x` will be borrowed twice, and compile failed, rustc don't process it.
          quote_spanned! { *span => {
            let mut _ಠ_ಠ = #ty::declarer();
            #(_ಠ_ಠ = _ಠ_ಠ #fields;)*
            _ಠ_ಠ.finish(ctx!())
          }}
          .to_tokens(tokens);
        }
      }
      ObjType::Var(var) => {
        if !self.children.is_empty() && fields.is_empty() {
          // If a variable node is declared with children, it cannot be used by others.
          // Therefore, there's no need to extend the built-in ability to it.
          var.to_tokens(tokens);
        } else if fields.is_empty() {
          quote_spanned! { var.span() => FatObj::new(#var) }.to_tokens(tokens);
        } else {
          let fields = fields.iter();
          // move `var` last, so that we can use it in the fields.
          quote_spanned! { var.span() => {
            let mut _ಠ_ಠ = FatObj::new(());
            #(_ಠ_ಠ = _ಠ_ಠ #fields;)*
            _ಠ_ಠ.map(move |_| #var)
          }}
          .to_tokens(tokens);
        }
      }
    }
  }
}
