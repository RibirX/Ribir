use crate::{
  rdl_macro::{DeclareField, RdlParent, StructLiteral},
  widget_macro::{WIDGETS, WIDGET_OF_BUILTIN_FIELD},
};
use inflector::Inflector;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use smallvec::SmallVec;
use syn::{
  parse_str,
  spanned::Spanned,
  token::{Comma, Paren},
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

    let compose_child = |tokens: &mut TokenStream| {
      this.to_tokens(tokens);
      children.iter().for_each(|c| {
        quote_spanned! { c.span() => .with_child(#c, ctx!())}.to_tokens(tokens);
      });
    };

    if builtin.is_empty() {
      compose_child(tokens);
    } else {
      let mut builtin_names: SmallVec<[Ident; 1]> = <_>::default();
      let mut builtin_types: SmallVec<[Path; 1]> = <_>::default();
      let mut fat_obj_init: SmallVec<[Ident; 1]> = <_>::default();

      WIDGETS
        .iter()
        .filter(|b_widget| builtin.contains_key(b_widget.ty))
        .for_each(|b| {
          let ty = &b.ty;
          let name = ty.to_snake_case();
          let ty = parse_str::<Path>(ty).unwrap();
          builtin_names.push(Ident::new(&name, ty.span()));
          builtin_types.push(ty);
          fat_obj_init.push(Ident::new(&format!("with_{}", name), name.span()));
        });

      let mut builtin_objs = WIDGETS
        .iter()
        .filter_map(|b| builtin.get(&b.ty))
        .zip(builtin_types.iter())
        .map(|(fields, ty)| {
          // 'b is live longer than 'a, safe convert, but we can't directly convert
          // `SmallVec<[&'b DeclareField; 1]>` to `SmallVec<[&'a DeclareField;
          // 1]>`, because `SmallVec<T>` is invariant over `T`.
          fn shorter_lifetime<'a, 'b: 'a>(
            fields: SmallVec<[&'b DeclareField; 1]>,
          ) -> SmallVec<[&'a DeclareField; 1]> {
            unsafe { std::mem::transmute(fields.clone()) }
          }
          let fields = shorter_lifetime(fields.clone());
          ObjNode::Obj { ty, span: *span, fields }
        });

      let p = builtin_names.first().unwrap();
      let mut compose_tokens = quote!();
      recursive_compose_with(
        p,
        builtin_names.iter().skip(1),
        &mut compose_tokens,
        |tokens| quote!(host).to_tokens(tokens),
      );

      if children.is_empty() {
        // Declare a struct for the `declare!` macro according to user
        // declaration.
        quote_spanned! { *span =>
          FatObj::new(#this)
            #(.#fat_obj_init(#builtin_objs))*
        }
        .to_tokens(tokens);
      } else {
        let p = builtin_objs.next().unwrap();
        recursive_compose_with(p, builtin_objs, tokens, compose_child);
      }
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
