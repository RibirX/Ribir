use crate::declare_func_derive::{DeclareCtx, DeclareMacro, FollowOn};
use proc_macro::{Diagnostic, Level};
use proc_macro2::{Span, TokenStream};

use quote::quote;
use syn::Ident;

#[derive(Debug)]
pub struct FollowInfo {
  pub widget: Ident,
  pub member: Option<Ident>,
  pub on: FollowOn,
}
#[derive(Debug)]
pub enum DeclareError {
  DuplicateID([Ident; 2]),
  CircleInit(Box<[FollowInfo]>),
  CircleFollow(Box<[FollowInfo]>),
  UnnecessarySkipNc(Span),
  DataFlowNoDepends(Span),
  KeyDependsOnOther {
    key: Span,
    depends_on: Vec<Span>,
  },
  DependOnWrapWidgetWithIfGuard {
    wrap_name: Ident,
    wrap_def_pos: [Span; 3],
  },
}

pub type Result<T> = std::result::Result<T, DeclareError>;

impl DeclareError {
  pub fn into_compile_error(&self, ctx: &DeclareCtx, declare: &DeclareMacro) -> TokenStream {
    self.error_emit(&ctx, declare);
    // A Valid widget return to avoid compile noise when error occur.
    quote! {{
      struct __Tmp;
      impl CombinationWidget for __Tmp {
        fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
          unreachable!();
          }
      }
      __Tmp.box_it()
    }}
  }

  pub fn error_emit(&self, ctx: &DeclareCtx, declare: &DeclareMacro) {
    let mut diagnostic = Diagnostic::new(Level::Error, "");
    match self {
      DeclareError::DuplicateID([id1, id2]) => {
        assert_eq!(id1, id2);
        diagnostic.set_spans(vec![id1.span().unwrap(), id2.span().unwrap()]);
        diagnostic.set_message(format!(
          "Same id(`{}`) assign to multiple widgets, id must be unique.",
          id1
        ));
      }
      DeclareError::CircleInit(path) => {
        let (msg, spans, note_spans) = path_info(path, ctx);
        let msg = format!("Can't init widget because circle follow: {}", msg);
        diagnostic.set_spans(spans);
        diagnostic.set_message(msg);
        let note_msg = "If the circular is your want and will finally not infinite change,\
        you should break the init circle and declare some follow relationship in `data_flow`, \
        and remember use `#[skip_nc]` attribute to skip the no change trigger of the field modify\
        to ignore infinite state change trigger.";
        diagnostic = diagnostic.span_note(note_spans, note_msg);
      }
      DeclareError::CircleFollow(path) => {
        let (msg, spans, note_spans) = path_info(path, ctx);
        let msg = format!(
          "Circle follow will cause infinite state change trigger: {}",
          msg
        );
        diagnostic.set_spans(spans);
        diagnostic.set_message(msg);
        let note_msg = "If the circular is your want, use `#[skip_nc]` attribute before field \
        or data_flow to skip the no change trigger of the field modify to ignore infinite state \
        change trigger.";
        diagnostic = diagnostic.span_note(note_spans, note_msg);
      }
      DeclareError::UnnecessarySkipNc(span) => {
        diagnostic.set_spans(vec![span.unwrap()]);
        diagnostic.set_message("Unnecessary attribute, because not depends on any others");
        diagnostic = diagnostic.help("Try to remove it.");
      }
      DeclareError::DataFlowNoDepends(span) => {
        diagnostic.set_spans(vec![span.unwrap()]);
        diagnostic.set_message("Declared a data flow but not depends on any others.");
        diagnostic = diagnostic.help("Try to remove it.");
      }
      DeclareError::KeyDependsOnOther { key, depends_on } => {
        let mut spans = vec![key.unwrap()];
        spans.extend(depends_on.into_iter().map(|s| s.unwrap()));
        diagnostic.set_spans(spans);
        diagnostic.set_message("The key attribute is not allowed to depend on others.");
      }
      DeclareError::DependOnWrapWidgetWithIfGuard { wrap_name, wrap_def_pos } => {
        let error_spans = declare
          .widget
          .all_syntax_fields()
          .filter_map(|f| {
            f.follows
              .as_ref()
              .and_then(|follows| follows.into_iter().find(|f| &f.widget == wrap_name))
          })
          .map(|f| f.spans.into_iter().map(|s| s.unwrap()))
          .flatten()
          .collect::<Vec<_>>();
        diagnostic.set_spans(error_spans);
        diagnostic.set_message( "Depends on a field which behind `if guard`, its existence depends on the `if guard` result in runtime.");
        diagnostic = diagnostic.span_help(
          wrap_def_pos.iter().map(|s| s.unwrap()).collect::<Vec<_>>(),
          "field define here.",
        );
      }
    };

    diagnostic.emit();
  }
}

// return a tuple compose by the string display of path, the path follow spans
// and the spans of will `#[skip_nc]` can add.
fn path_info(
  path: &Box<[FollowInfo]>,
  ctx: &DeclareCtx,
) -> (String, Vec<proc_macro::Span>, Vec<proc_macro::Span>) {
  let path = path.iter().map(|FollowInfo { widget, member, on }| {
    let widget = ctx.widget_name_to_id(widget);
    (widget, member, on)
  });
  let msg = path
    .clone()
    .map(|(widget, member, on)| {
      let on_widget = ctx.widget_name_to_id(&on.widget);
      if let Some(m) = member {
        format!("{}.{} ～> {} ", widget, m, on_widget)
      } else {
        format!("{} ～> {} ", widget, on_widget)
      }
    })
    .collect::<Vec<_>>()
    .join(", ");

  let spans = path.clone().fold(vec![], |mut res, (widget, member, on)| {
    // extra add the widget define position, tell user will the id define.
    let src_def = ctx
      .named_widgets
      .get(widget)
      .expect("id must in named widgets");
    res.push(src_def.span().unwrap());

    res.push(widget.span().unwrap());
    if let Some(m) = member {
      res.push(m.span().unwrap());
    }

    let t_spans = on.spans.iter().map(|s| s.unwrap());
    res.extend(t_spans);
    res
  });

  let note_spans = path
    .map(|(widget, member, on)| {
      if let Some(m) = member {
        m.span().unwrap()
      } else {
        on.spans
          .into_iter()
          .fold(widget.span(), |s1, s2| s2.join(s1).unwrap())
          .unwrap()
      }
    })
    .collect::<Vec<_>>();

  (msg, spans, note_spans)
}
