use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, Expr, Ident};

mod code_gen;
mod desugar;
mod parser;
pub use desugar::Desugared;
pub use parser::{MacroSyntax, TrackField};
mod visit_mut;
pub use visit_mut::*;
mod name_used_info;
pub use name_used_info::*;
mod variable_names;
pub use variable_names::*;

use crate::error::DeclareError;

use self::desugar::{builtin_obj, NamedObj};

pub mod kw {
  syn::custom_keyword!(widget);
  syn::custom_keyword!(track);
  syn::custom_keyword!(DynWidget);
  syn::custom_keyword!(id);
  syn::custom_keyword!(skip_nc);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(State);
  syn::custom_keyword!(Transition);
  syn::custom_punctuation!(FlowArrow, ~>);
  syn::custom_keyword!(on);
  syn::custom_keyword!(transition);
  syn::custom_keyword!(change);
}

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
    declare_objs: desugar
      .named_objs
      .objs()
      .map(|obj| (obj.name().clone(), obj.ty().clone()))
      .collect(),
    track_names: desugar
      .track
      .iter()
      .flat_map(|t| t.track_externs.iter().map(|sf| sf.member.clone()))
      .collect(),
    ..<_>::default()
  };
  if let Some(ref outside_ctx) = outside_ctx {
    ctx.declare_objs.extend(outside_ctx.declare_objs.clone());
    ctx.track_names.extend(outside_ctx.track_names.clone());
    ctx.analyze_stack = outside_ctx.analyze_stack.clone();
  };

  ctx.visit_desugared_syntax_mut(&mut desugar);
  desugar.collect_warnings(&ctx);
  let used_widgets = ctx.used_objs;
  used_widgets
    .iter()
    .for_each(|(name, UsedInfo { builtin, spans })| {
      // add default builtin widget, which used by others but but declared.
      if let Some(builtin) = builtin {
        if !desugar.named_objs.contains(name) && desugar.named_objs.contains(&builtin.src_name) {
          let BuiltinUsed { src_name, builtin_ty } = builtin;
          let obj = builtin_obj(src_name, builtin_ty, <_>::default());
          desugar.add_named_builtin_obj(src_name, obj);
        }
      }

      if let Some(obj) = desugar.named_objs.get_mut(name) {
        // named obj used by other should force be stateful
        match obj {
          NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => obj.stateful = true,
          NamedObj::DuplicateListener { objs, .. } => {
            desugar.errors.push(DeclareError::DependsOnDupListener {
              declare_at: objs.iter().map(|o| o.name.span().unwrap()).collect(),
              used_at: spans.clone(),
            })
          }
        }
      }
    });

  let mut tokens = quote! {};
  desugar.circle_detect();
  desugar.to_tokens(&mut tokens);

  if let Some(outside_ctx) = outside_ctx {
    let used_outsides = used_widgets
      .iter()
      .filter(|(name, _)| {
        !desugar.named_objs.contains(name)
          && desugar.track.as_ref().map_or(true, |track| {
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
        UsedType::MOVE_CAPTURE,
      )
    });
  }

  tokens.into()
}

impl TrackExpr {
  pub fn new(expr: Expr) -> Self { Self { expr, used_name_info: <_>::default() } }

  pub fn upstream_tokens(&self) -> Option<TokenStream> {
    self
      .used_name_info
      .directly_used_widgets()
      .map(|directly_used| {
        let upstream = directly_used.clone().map(|w| {
          quote_spanned! { w.span() => #w.raw_change_stream() }
        });
        if directly_used.count() > 1 {
          quote! {  observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
        } else {
          quote! { #(#upstream)* }
        }
      })
  }
}

impl ToTokens for TrackExpr {
  fn to_tokens(&self, tokens: &mut TokenStream) { self.expr.to_tokens(tokens) }
}
