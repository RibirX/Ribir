use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, Expr, Ident};

mod code_gen;
mod desugar;
mod parser;
pub use desugar::Desugared;
pub use parser::{MacroSyntax, StateField};
mod visit_mut;
pub use visit_mut::*;
mod name_used_info;
pub use name_used_info::*;
mod variable_names;
pub use variable_names::*;

use crate::error::DeclareError;

use self::desugar::{builtin_obj, NamedObj};

fn capture_widget(widget: &Ident) -> TokenStream {
  quote_spanned!(widget.span() => let #widget = #widget.clone_stateful();)
}

#[derive(Debug, Clone)]
pub struct TrackExpr {
  pub expr: Expr,
  pub used_name_info: ScopeUsedInfo,
}

pub fn gen_widget_macro(
  input: proc_macro::TokenStream,
  outside_ctx: Option<&mut VisitCtx>,
) -> proc_macro::TokenStream {
  let macro_syntax = parse_macro_input! { input as MacroSyntax };
  let mut desugar = macro_syntax.desugar();

  let mut ctx = VisitCtx {
    track_names: desugar
      .states
      .iter()
      .flat_map(|t| t.states.iter().map(|sf| sf.member.clone()))
      .collect(),
    ..<_>::default()
  };

  if let Some(ref outside_ctx) = outside_ctx {
    ctx.declare_objs.extend(outside_ctx.declare_objs.clone());
    ctx.track_names.extend(outside_ctx.track_names.clone());
    ctx.analyze_stack = outside_ctx.analyze_stack.clone();
  };

  // visit init without named objects.
  if let Some(init) = desugar.init.as_mut() {
    ctx.visit_init_stmts_mut(init);
    if let Some(ctx_name) = init.ctx_name.clone() {
      ctx.track_names.insert(ctx_name);
    }
  }

  ctx.declare_objs.extend(
    desugar
      .named_objs
      .objs()
      .map(|obj| (obj.name().clone(), obj.ty().clone())),
  );

  ctx.visit_desugared_syntax_mut(&mut desugar);
  if let Some(ctx_name) = desugar.init.as_ref().and_then(|i| i.ctx_name.as_ref()) {
    if let Some(used_info) = ctx.used_objs.get(ctx_name) {
      desugar.errors.push(DeclareError::CtxOnlyAllowInInit {
        name: ctx_name.to_string(),
        spans: used_info.spans.clone(),
      })
    }
  }

  ctx
    .used_objs
    .iter()
    .for_each(|(name, UsedInfo { builtin, .. })| {
      // add default builtin widget, which used by others but but declared.
      if let Some(builtin) = builtin {
        if !desugar.named_objs.contains(name) && desugar.named_objs.contains(&builtin.src_name) {
          let BuiltinUsed { src_name, builtin_ty } = builtin;
          let obj = builtin_obj(src_name, builtin_ty, <_>::default());
          desugar.add_named_builtin_obj(src_name.clone(), obj);
        }
      }

      if let Some(obj) = desugar.named_objs.get_mut(name) {
        // named obj used by other should force be stateful
        match obj {
          NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => obj.stateful = true,
        }
      }
    });

  desugar.collect_warnings(&ctx);
  let mut tokens = quote! {};
  desugar.circle_detect();
  desugar.gen_tokens(&mut tokens, &ctx);

  if let Some(outside_ctx) = outside_ctx {
    let used_outsides = ctx
      .used_objs
      .iter()
      .filter(|(name, _)| {
        !desugar.named_objs.contains(name)
          && desugar.states.as_ref().map_or(true, |track| {
            track.track_names().find(|n| n == name).is_none()
          })
      })
      .collect::<Vec<_>>();
    if !used_outsides.is_empty() {
      let captures = used_outsides.iter().map(|(name, _)| capture_widget(name));
      tokens = quote! {{
        #(#captures)*
        #tokens
      }};
    }
    used_outsides.into_iter().for_each(|(name, used_info)| {
      outside_ctx.add_used_widget(
        name.clone(),
        used_info.builtin.clone(),
        UsedType::SCOPE_CAPTURE,
      )
    });
  }

  tokens.into()
}

impl TrackExpr {
  pub fn new(expr: Expr) -> Self { Self { expr, used_name_info: <_>::default() } }
}

impl ToTokens for TrackExpr {
  fn to_tokens(&self, tokens: &mut TokenStream) { self.expr.to_tokens(tokens) }
}
