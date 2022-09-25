use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
  braced,
  parse::Parse,
  spanned::Spanned,
  token::{self, Brace, Pound},
  visit_mut::VisitMut,
  Expr, Ident, Path,
};

use super::{
  animations::Animations,
  child_variable,
  declare_widget::{assign_uninit_field, try_parse_skip_nc},
  kw,
  on_change::{ChangeFlow, OnChangeDo},
  on_event_do::OnEventDo,
  track::Track,
  DeclareCtx, DeclareWidget, Id, IdType, ObjectUsed, ScopeUsedInfo,
};
use crate::{
  error::{CircleUsedPath, DeclareError, DeclareWarning},
  widget_attr_macro::{ctx_ident, ribir_variable, ObjectUsedPath, UsedType},
};

pub const EXPR_WIDGET: &str = "ExprWidget";
pub const EXPR_FIELD: &str = "expr";

#[derive(Debug, Clone)]
pub struct TrackExpr {
  pub expr: Expr,
  pub used_name_info: ScopeUsedInfo,
}

pub struct WidgetMacro {
  widget: DeclareWidget,
  track: Option<Track>,
  animations: Option<Animations>,
  on_items: Vec<OnItem>,
}

pub enum OnItem {
  OnEvent(OnEventDo),
  OnChange(OnChangeDo),
}

pub fn is_expr_keyword(ty: &Path) -> bool { ty.get_ident().map_or(false, |ty| ty == EXPR_WIDGET) }

impl Parse for WidgetMacro {
  fn parse(content: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut widget: Option<DeclareWidget> = None;
    let mut items = vec![];
    let mut animations: Option<Animations> = None;
    let mut track: Option<Track> = None;
    loop {
      if content.is_empty() {
        break;
      }
      let lk = content.lookahead1();
      if lk.peek(Pound) || lk.peek(kw::on) {
        items.push(content.parse()?);
      } else if lk.peek(kw::animations) {
        let a = content.parse()?;
        assign_uninit_field!(animations, a, animations)?;
      } else if lk.peek(kw::track) {
        let mut t = content.parse::<Track>()?;
        if let Some(ot) = track.take() {
          t.track_externs.extend(ot.track_externs);
        }
        track = Some(t);
      } else if lk.peek(Ident) && content.peek2(token::Brace) {
        let w: DeclareWidget = content.parse()?;
        if let Some(first) = widget.as_ref() {
          let err = syn::Error::new(
            w.span(),
            &format!(
              "Only one root widget can declare, but `{}` already declared.",
              first.path.to_token_stream()
            ),
          );
          return Err(err);
        }
        widget = Some(w);
      } else {
        return Err(lk.error());
      }
    }

    let widget =
      widget.ok_or_else(|| syn::Error::new(content.span(), "must declare widget in `widget!`"))?;

    Ok(Self {
      widget,
      on_items: items,
      animations,
      track,
    })
  }
}

impl Parse for TrackExpr {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(Self {
      expr: input.parse()?,
      used_name_info: <_>::default(),
    })
  }
}

impl Parse for OnItem {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.peek(Pound) {
      let flow: ChangeFlow = input.parse()?;
      Ok(OnItem::OnChange(flow.into_change_do()))
    } else if input.peek2(Ident) && input.peek3(Brace) {
      Ok(OnItem::OnEvent(input.parse::<OnEventDo>()?))
    } else {
      let on_token: kw::on = input.parse()?;
      let expr: TrackExpr = input.parse()?;
      let lk = input.lookahead1();
      if lk.peek(kw::FlowArrow) {
        let flow = ChangeFlow {
          skip_nc: None,
          on_token,
          from: expr,
          flow_arrow: input.parse()?,
          to: input.parse()?,
        };
        Ok(OnItem::OnChange(flow.into_change_do()))
      } else if lk.peek(Brace) {
        let content;
        let change_do = OnChangeDo {
          on_token,
          observe: expr,
          brace: braced!( content in input),
          skip_nc: try_parse_skip_nc(&content)?,
          change_token: content.parse()?,
          colon_token: content.parse()?,
          subscribe_do: content.parse()?,
        };
        Ok(OnItem::OnChange(change_do))
      } else {
        Err(lk.error())
      }
    }
  }
}

impl WidgetMacro {
  pub fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> TokenStream {
    fn circle_stack_to_path(stack: &[ObjectUsedPath], ctx: &DeclareCtx) -> Box<[CircleUsedPath]> {
      stack.iter().map(|c| c.to_used_path(ctx)).collect()
    }

    self.id_collect(ctx);
    ctx.visit_widget_macro_mut(self);

    self.widget.traverses_widget().for_each(|w| {
      if let Some(name) = w.name() {
        w.builtin.add_user_perspective_pairs(name, ctx);
      }
    });

    self.on_items.iter().for_each(|item| item.error_check(ctx));

    let mut tokens = quote!();
    // named object define.
    if !ctx.named_objects.is_empty() {
      let mut follows = self.analyze_object_dependencies();
      // init circle check
      Self::circle_check(&follows, |stack| {
        ctx
          .errors
          .push(DeclareError::CircleInit(circle_stack_to_path(stack, ctx)));
      });

      // change flow should not effect the named object init order, and we allow
      // circle follow with skip_nc attribute. So we add the data flow
      // relationship and individual check the circle follow error.
      if !self.on_items.is_empty() {
        self
          .on_items
          .iter()
          .for_each(|item| item.analyze_observe_depends(&mut follows));
        // circle dependencies check
        Self::circle_check(&follows, |stack| {
          let weak_depends = stack
            .iter()
            .any(|s| !s.used_info.used_type.contains(UsedType::USED) || s.skip_nc_cfg);

          if !weak_depends {
            ctx
              .errors
              .push(DeclareError::CircleFollow(circle_stack_to_path(stack, ctx)))
          }
        });
      }

      let mut named_widgets_def = self.named_objects_def_tokens(ctx);

      Self::deep_used_iter(&follows, |name| {
        tokens.extend(named_widgets_def.remove(name));
      });

      named_widgets_def
        .into_values()
        .for_each(|def_tokens| tokens.extend(def_tokens));
    }

    self.all_anonyms_widgets_tokens(ctx, &mut tokens);
    for item in self.on_items.iter() {
      item.gen_tokens(&mut tokens, ctx)
    }

    if let Some(a) = self.animations.as_mut() {
      a.gen_tokens(ctx, &mut tokens);
    }
    self.compose_tokens(ctx, &mut tokens);

    let ctx_name = ctx_ident(Span::call_site());
    let name = self.widget_identify();
    let mut tokens = quote! {
      (move |#ctx_name: &mut BuildCtx| {
        #tokens
        #name.into_widget()
      }).into_widget()
    };

    let track = self.track.as_ref();
    if track.map_or(false, Track::has_def_names) {
      tokens = quote! {{
        #track
        #tokens
      }};
    }

    if ctx.errors.is_empty() {
      ctx
        .unused_id_warning()
        .chain(self.widget.warnings())
        .chain(self.on_items.iter().filter_map(|i| i.warning()))
        .for_each(|w| w.emit_warning());
      tokens
    } else {
      let errs = ctx.errors.drain(..).map(|e| e.into_compile_error());
      quote! {#(#errs)*}
    }
  }

  pub fn id_collect(&self, ctx: &mut DeclareCtx) {
    for w in self.widget.traverses_widget() {
      if let Some(Id { name, .. }) = w.named.as_ref() {
        ctx.add_named_obj(name.clone(), IdType::DECLARE);
        w.builtin.collect_names(name, ctx);
      }
    }
    for name in self.animations.iter().flat_map(|a| a.names()) {
      ctx.add_named_obj(name.clone(), IdType::DECLARE);
    }
    for name in self.track.iter().flat_map(|t| t.track_names()) {
      ctx.add_named_obj(name.clone(), IdType::USER_SPECIFY);
    }
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  fn analyze_object_dependencies(&self) -> BTreeMap<Ident, ObjectUsed> {
    let mut follows = self.widget.analyze_object_dependencies();

    if let Some(animations) = self.animations.as_ref() {
      follows.extend(animations.dependencies());
    }
    follows
  }

  // return the key-value map of the named widget define tokens.
  fn named_objects_def_tokens(&self, ctx: &DeclareCtx) -> HashMap<Ident, TokenStream, RandomState> {
    self
      .widget
      .traverses_widget()
      .filter_map(|w| {
        w.name()
          .map(|name| w.host_and_builtin_widgets_tokens(name, ctx))
      })
      .flatten()
      .chain(
        self
          .animations
          .as_ref()
          .map(|a| a.named_objects_def_tokens_iter(ctx))
          .into_iter()
          .flatten(),
      )
      .collect()
  }

  pub fn all_anonyms_widgets_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    fn anonyms_widgets_tokens(
      name: &Ident,
      w: &DeclareWidget,
      ctx: &DeclareCtx,
      tokens: &mut TokenStream,
    ) {
      if w.name().is_none() {
        w.host_and_builtin_widgets_tokens(name, ctx)
          .for_each(|(_, widget)| tokens.extend(widget));
      }

      w.children.iter().enumerate().for_each(|(idx, c)| {
        if let Some(name) = c.name() {
          anonyms_widgets_tokens(&name, c, ctx, tokens);
        } else {
          anonyms_widgets_tokens(&child_variable(name, idx), c, ctx, tokens);
        }
      })
    }
    let name = self.widget_identify();
    anonyms_widgets_tokens(&name, &self.widget, ctx, tokens)
  }

  pub fn compose_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    fn compose_widget(w: &DeclareWidget, name: &Ident, ctx: &DeclareCtx, tokens: &mut TokenStream) {
      let mut compose_children = quote! {};
      w.children.iter().enumerate().for_each(|(idx, c)| {
        let c_name = c
          .name()
          .cloned()
          .unwrap_or_else(|| child_variable(name, idx));

        // deep compose first.
        compose_widget(c, &c_name, ctx, tokens);
        compose_children.extend(quote_spanned! { c.span() => .have_child(#c_name) });
      });
      if !compose_children.is_empty() {
        let hint = (w.children.len() > 1).then(|| quote! {: MultiChildWidget<_>});
        tokens.extend(quote! { let #name #hint = #name #compose_children; });
      }

      w.builtin.compose_tokens(name, ctx, tokens);
    }

    let name = self.widget_identify();
    compose_widget(&self.widget, &name, ctx, tokens);
  }

  pub fn widget_identify(&self) -> Ident {
    if let Some(name) = self.widget.name() {
      name.clone()
    } else {
      ribir_variable("ribir", self.widget.path.span())
    }
  }

  fn circle_check(
    follow_infos: &BTreeMap<Ident, ObjectUsed>,
    mut err_detect: impl FnMut(&[ObjectUsedPath]),
  ) {
    #[derive(PartialEq, Eq, Debug)]
    enum CheckState {
      Checking,
      Checked,
    }

    let mut check_info: HashMap<_, _, RandomState> = HashMap::default();
    let mut stack = vec![];

    // return if the widget follow contain circle.
    fn widget_follow_circle_check<'a>(
      name: &'a Ident,
      follow_infos: &'a BTreeMap<Ident, ObjectUsed>,
      check_info: &mut HashMap<&'a Ident, CheckState, RandomState>,
      stack: &mut Vec<ObjectUsedPath<'a>>,
      err_detect: &mut impl FnMut(&[ObjectUsedPath]),
    ) {
      match check_info.get(name) {
        None => {
          if let Some(follows) = follow_infos.get(name) {
            follows.used_full_path_iter(name).for_each(|path| {
              let next_obj = path.used_obj;
              check_info.insert(name, CheckState::Checking);
              stack.push(path);
              widget_follow_circle_check(next_obj, follow_infos, check_info, stack, err_detect);
              stack.pop();
            });
            debug_assert_eq!(check_info.get(name), Some(&CheckState::Checking));
            check_info.insert(name, CheckState::Checked);
          };
        }
        Some(CheckState::Checking) => {
          let start = stack.iter().position(|v| v.obj == name).unwrap();
          err_detect(&stack[start..]);
        }
        Some(CheckState::Checked) => {}
      };
    }

    follow_infos.keys().for_each(|name| {
      widget_follow_circle_check(
        name,
        follow_infos,
        &mut check_info,
        &mut stack,
        &mut err_detect,
      );
    });
  }

  fn deep_used_iter<F: FnMut(&Ident)>(follows: &BTreeMap<Ident, ObjectUsed>, mut callback: F) {
    // circular may exist widget attr follow widget self to init.
    let mut stacked = std::collections::HashSet::<_, RandomState>::default();

    let mut stack = follows.keys().rev().collect::<Vec<_>>();
    while let Some(w) = stack.pop() {
      match follows.get(w) {
        Some(f) if !stacked.contains(w) => {
          stack.push(w);
          stack.extend(f.used_obj_iter());
        }
        _ => callback(w),
      }
      stacked.insert(w);
    }
  }
}

impl DeclareCtx {
  pub fn visit_widget_macro_mut(&mut self, d: &mut WidgetMacro) {
    self.visit_declare_widget_mut(&mut d.widget);
    d.on_items
      .iter_mut()
      .for_each(|item| self.visit_on_item(item));

    if let Some(animations) = d.animations.as_mut() {
      self.visit_animations_mut(animations);
    }
  }

  pub fn visit_track_expr(&mut self, expr: &mut TrackExpr) {
    self.visit_expr_mut(&mut expr.expr);
    expr.used_name_info = self.take_current_used_info();
  }
}

impl TrackExpr {
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

  pub fn used_nothing_warning(&self) -> Option<DeclareWarning> {
    let Self { expr, used_name_info } = self;
    used_name_info
      .directly_used_widgets()
      .is_none()
      .then(|| DeclareWarning::ObserveIsConst(expr.span().unwrap()))
  }
}

impl ToTokens for TrackExpr {
  fn to_tokens(&self, tokens: &mut TokenStream) { self.expr.to_tokens(tokens) }
}

impl OnItem {
  fn warning(&self) -> Option<DeclareWarning> {
    match self {
      OnItem::OnEvent(e) => e.warning(),
      OnItem::OnChange(c) => c.warning(),
    }
  }

  pub fn error_check(&self, ctx: &mut DeclareCtx) {
    if let OnItem::OnEvent(e) = self {
      e.error_check(ctx);
    }
  }

  fn gen_tokens(&self, tokens: &mut TokenStream, ctx: &mut DeclareCtx) {
    match self {
      OnItem::OnEvent(e) => e.gen_tokens(tokens, ctx),
      OnItem::OnChange(c) => c.to_tokens(tokens),
    }
  }

  fn analyze_observe_depends<'a>(&'a self, depends: &mut BTreeMap<Ident, ObjectUsed<'a>>) {
    match self {
      OnItem::OnEvent(e) => e.analyze_observe_depends(depends),
      OnItem::OnChange(c) => c.analyze_observe_depends(depends),
    }
  }
}

impl DeclareCtx {
  pub fn visit_on_item(&mut self, item: &mut OnItem) {
    match item {
      OnItem::OnEvent(e) => self.visit_on_event_do(e),
      OnItem::OnChange(c) => self.visit_on_change_do(c),
    }
  }
}
