use crate::{
  rdl_macro::{DeclareField, RdlParent, StructLiteral},
  widget_macro::{ribir_variable, WIDGETS, WIDGET_OF_BUILTIN_FIELD},
};
use inflector::Inflector;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::{quote_spanned, ToTokens};
use smallvec::SmallVec;
use syn::{
  parse_str,
  spanned::Spanned,
  token::{Brace, Comma, Paren},
  Ident, Macro, Path,
};

pub struct DeclareObj<'a> {
  this: ObjNode<'a>,
  span: Span,
  builtin: ahash::HashMap<&'static str, SmallVec<[&'a DeclareField; 1]>>,
  children: &'a Vec<Macro>,
}
enum ObjNode<'a> {
  Obj {
    span: Span,
    ty: &'a Path,
    fields: SmallVec<[&'a DeclareField; 1]>,
  },
  Var(&'a Ident),
}

impl<'a> DeclareObj<'a> {
  pub fn from_literal(mac: &'a StructLiteral) -> Result<Self, TokenStream> {
    let StructLiteral { parent, brace, fields, children } = mac;
    let mut builtin: ahash::HashMap<_, SmallVec<[&DeclareField; 1]>> = <_>::default();
    let span = match parent {
      RdlParent::Type(ty) => ty.span(),
      RdlParent::Var(name) => name.span(),
    };
    let span = span.join(brace.span).unwrap();

    let this = match parent {
      RdlParent::Type(ty) => {
        let mut self_fields = SmallVec::default();
        for f in fields {
          if let Some(ty) = WIDGET_OF_BUILTIN_FIELD
            .get(f.member.to_string().as_str())
            .filter(|builtin_ty| !ty.is_ident(builtin_ty))
          {
            builtin.entry(*ty).or_default().push(f);
          } else {
            self_fields.push(f)
          }
        }
        ObjNode::Obj { ty, fields: self_fields, span }
      }
      RdlParent::Var(name) => {
        for f in fields {
          if let Some(ty) = WIDGET_OF_BUILTIN_FIELD.get(f.member.to_string().as_str()) {
            builtin.entry(*ty).or_default().push(f);
          } else {
            return Err(quote_spanned! { f.span() =>
              compile_error!("Not allow to declare a field of a variable parent.")
            });
          }
        }
        ObjNode::Var(name)
      }
    };

    Ok(Self { this, span, builtin, children })
  }
}

impl<'a> ToTokens for DeclareObj<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { this, span, builtin, children } = self;

    // if children is empty, we declare a `FatObj`, so it's can be used and
    // referenced by others, otherwise directly composed.

    if children.is_empty() && builtin.is_empty() {
      quote_spanned! { *span => FatObj::new(#this) }.to_tokens(tokens)
    } else {
      Brace::default().surround(tokens, |tokens| {
        let mut builtin_names = vec![];
        for (ty_str, fields) in builtin {
          // 'b is live longer than 'a, safe convert, but we can't directly convert
          // `SmallVec<[&'b DeclareField; 1]>` to `SmallVec<[&'a DeclareField;
          // 1]>`, because `SmallVec<T>` is invariant over `T`.
          fn shorter_lifetime<'a, 'b: 'a>(
            fields: SmallVec<[&'b DeclareField; 1]>,
          ) -> SmallVec<[&'a DeclareField; 1]> {
            unsafe { std::mem::transmute(fields.clone()) }
          }
          let fields = shorter_lifetime(fields.clone());

          let builtin_span = fields[0].span();
          let ty = parse_str::<Path>(ty_str).unwrap();
          let obj = ObjNode::Obj { ty: &ty, span: builtin_span, fields };

          let snaked_ty_str = ty_str.to_snake_case();
          let name = Ident::new(&snaked_ty_str, builtin_span);
          quote_spanned! { builtin_span => let #name = #obj; }.to_tokens(tokens);
          builtin_names.push(name);
        }

        let mut children_names = vec![];
        for (i, c) in children.iter().enumerate() {
          let child = ribir_variable(&format!("child_{i}"), c.span());
          quote_spanned! { c.span() => let #child = #c; }.to_tokens(tokens);
          children_names.push(child)
        }

        if children.is_empty() {
          let builtin_init = builtin_names
            .iter()
            .map(|name| Ident::new(&format!("with_{name}"), name.span()));

          quote_spanned! {
            *span =>FatObj::new(#this)#(.#builtin_init(#builtin_names))*
          }
          .to_tokens(tokens);
        } else {
          // todo: tmp code, we should use FatObj to compose builtin widgets in every
          // where, so we can keep the builtin widget compose order consistent.
          builtin_names.sort_by_key(|name| {
            WIDGETS
              .iter()
              .position(|b| *name == b.ty.to_snake_case())
              .unwrap()
          });
          if !builtin.is_empty() {
            let first = &builtin_names[0];
            let rest_builtin = &builtin_names[1..];

            recursive_compose_with(first, rest_builtin.iter(), tokens, |tokens| {
              quote_spanned! { *span =>
                #this #(.with_child(#children_names, ctx!()))*
              }
              .to_tokens(tokens)
            });
          } else {
            quote_spanned! { *span =>
              #this #(.with_child(#children_names, ctx!()))*
            }
            .to_tokens(tokens)
          }
        }
      });
    }
  }
}

fn recursive_compose_with(
  p: impl ToTokens,
  mut child_chain: impl Iterator<Item = impl ToTokens>,
  tokens: &mut TokenStream,
  leaf: impl FnOnce(&mut TokenStream),
) {
  p.to_tokens(tokens);
  quote_spanned! { p.span() => .with_child}.to_tokens(tokens);
  Paren(p.span()).surround(tokens, |tokens| {
    let child = child_chain.next();
    if let Some(c) = child {
      recursive_compose_with(c, child_chain, tokens, leaf)
    } else {
      leaf(tokens)
    }
    Comma::default().to_tokens(tokens);
    quote! { ctx!() }.to_tokens(tokens);
  });
}

impl<'a> ToTokens for ObjNode<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      Self::Obj { ty, span, fields } => {
        quote_spanned! { *span => #ty::declare2_builder() }.to_tokens(tokens);
        fields.iter().for_each(|f| f.to_tokens(tokens));
        tokens.extend(quote_spanned! { *span => .build(ctx!()) });
      }
      Self::Var(var) => var.to_tokens(tokens),
    }
  }
}
