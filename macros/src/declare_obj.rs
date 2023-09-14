use crate::{
  rdl_macro::{DeclareField, RdlParent, StructLiteral},
  variable_names::{WIDGETS, WIDGET_OF_BUILTIN_FIELD},
};
use inflector::Inflector;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use smallvec::smallvec;
use smallvec::SmallVec;
use syn::{parse_str, spanned::Spanned, token::Brace, Ident, Macro, Path};

pub struct DeclareObj<'a> {
  span: Span,
  /// if declare a builtin widget, this is None. For example:
  ///   `@Margin { margin: ... }`
  this: Option<ObjNode<'a>>,
  builtin: Vec<(&'static str, SmallVec<[&'a DeclareField; 1]>)>,
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
    let StructLiteral { span, parent, fields, children } = mac;
    let span = *span;
    let mut builtin: Vec<(&'static str, SmallVec<[&'a DeclareField; 1]>)> = vec![];

    let mut self_fields = SmallVec::default();
    for f in fields {
      if let Some(ty) = WIDGET_OF_BUILTIN_FIELD.get(f.member.to_string().as_str()) {
        if let Some((_, fields)) = builtin.iter_mut().find(|(ty_name, _)| ty_name == ty) {
          fields.push(f);
        } else {
          builtin.push((*ty, smallvec![f]));
        }
      } else {
        self_fields.push(f);
      }
    }

    fn invalid_member_err(fields: &[&DeclareField], err_msg: &str) -> Result<(), TokenStream> {
      if fields.is_empty() {
        Ok(())
      } else {
        let mut err_tokens = quote! {};
        for f in fields {
          quote_spanned! { f.member.span() => #err_msg }.to_tokens(&mut err_tokens)
        }
        Err(err_tokens)
      }
    }

    match parent {
      RdlParent::Type(ty) => {
        if WIDGETS.iter().any(|w| ty.is_ident(w.ty)) {
          invalid_member_err(
            &self_fields,
            &format!("not a valid member of {}", ty.to_token_stream()),
          )?;
          Ok(Self { this: None, span, builtin, children })
        } else {
          let this = Some(ObjNode::Obj { ty, fields: self_fields, span });
          Ok(Self { this, span, builtin, children })
        }
      }
      RdlParent::Var(name) => {
        invalid_member_err(
          &self_fields,
          "only allow to declare builtin fields in a variable parent.",
        )?;
        let this = Some(ObjNode::Var(name));
        Ok(Self { this, span, builtin, children })
      }
    }
  }
}

impl<'a> ToTokens for DeclareObj<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    Brace(self.span).surround(tokens, |tokens| {
      match &self.this {
        Some(this @ ObjNode::Obj { span, .. }) => {
          // declare the host widget before builtin widget and children.
          // so that we can use variable if it moved in builtin widget and children.
          // this is consistent with the user's declaration.

          quote_spanned! { *span => let _ribir_ಠ_ಠ = #this; }.to_tokens(tokens);
          let name = Ident::new("_ribir_ಠ_ಠ", self.span);
          self.compose_builtin_and_children(&name, tokens)
        }
        Some(ObjNode::Var(var)) => self.compose_builtin_and_children(var, tokens),
        None => {
          let (builtin_names, children) = self.declare_builtin_objs_and_children(tokens);
          let built_obj = self.to_builtin_obj(builtin_names);
          quote_spanned! { self.span => #built_obj #(.with_child(#children, ctx!()))* }
            .to_tokens(tokens)
        }
      }
    })
  }
}

impl<'a> DeclareObj<'a> {
  /// declare the builtin inner widgets, and return the name of them.
  fn declare_builtin_objs_and_children(
    &self,
    tokens: &mut TokenStream,
  ) -> (SmallVec<[Ident; 1]>, SmallVec<[Ident; 1]>) {
    let mut builtin_names = smallvec![];
    for (ty_str, fields) in &self.builtin {
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
    let mut children_names = smallvec![];
    for (i, c) in self.children.iter().enumerate() {
      let child = Ident::new(&format!("_child_{i}_ಠ_ಠ"), c.span());
      quote_spanned! { c.span() => let #child = #c; }.to_tokens(tokens);
      children_names.push(child)
    }

    (builtin_names, children_names)
  }

  /// compose the builtin objs as `BuiltinObj`
  fn to_builtin_obj(&self, builtin_names: SmallVec<[Ident; 1]>) -> TokenStream {
    if builtin_names.is_empty() {
      quote_spanned! { self.span => BuiltinObj::default() }
    } else {
      let builtin_init = builtin_names
        .iter()
        .map(|name| Ident::new(&format!("set_builtin_{name}"), name.span()));
      quote_spanned! { self.span => BuiltinObj::default()#(.#builtin_init(#builtin_names))* }
    }
  }

  fn compose_builtin_and_children(&self, var: &Ident, tokens: &mut TokenStream) {
    let (builtin_names, children) = self.declare_builtin_objs_and_children(tokens);
    // if builtin is empty and children is not empty, we needn't to create a
    // `FatObj`, because not support to use the builtin widget in this case.
    if builtin_names.is_empty() && !children.is_empty() {
      quote_spanned! { self.span =>
        #var #(.with_child(#children, ctx!()))*
      }
      .to_tokens(tokens);
    } else {
      let built_obj = self.to_builtin_obj(builtin_names);
      quote_spanned! { self.span =>
        FatObj::new(#var, #built_obj)#(.with_child(#children, ctx!()))*
      }
      .to_tokens(tokens)
    }
  }
}

impl<'a> ToTokens for ObjNode<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      Self::Obj { ty, span, fields } => {
        quote_spanned! { *span => #ty::declare2_builder() }.to_tokens(tokens);
        fields.iter().for_each(|f| f.to_tokens(tokens));
        tokens.extend(quote_spanned! { *span => .build_declare(ctx!()) });
      }
      Self::Var(var) => var.to_tokens(tokens),
    }
  }
}
