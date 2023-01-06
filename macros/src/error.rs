use crate::widget_macro::NameUsedInfo;
use proc_macro::{Diagnostic, Level, Span};
use proc_macro2::TokenStream;

use quote::ToTokens;
use syn::Ident;

#[derive(Debug)]
pub struct CircleUsedPath {
  pub obj: Ident,
  pub member: Option<Ident>,
  pub used_widget: Ident,
  pub used_info: NameUsedInfo,
}

#[derive(Debug)]
pub enum DeclareError {
  DuplicateID([Ident; 2]),
  CircleDepends(Box<[CircleUsedPath]>),
  WatchNothing(Span),
  PropInvalidTarget(proc_macro2::Span),
  TransitionByConflict(Span),
  LetWatchWrongPlace(Span),
  SynErr(syn::Error),
}

#[derive(Debug)]
pub enum DeclareWarning {
  UnusedName(Span),
  DefObjWithoutId(Span),
}

impl DeclareError {
  pub fn to_compile_error(&self, tokens: &mut TokenStream) {
    let mut diagnostic = Diagnostic::new(Level::Error, "");
    match self {
      DeclareError::DuplicateID([id1, id2]) => {
        assert_eq!(id1, id2);
        diagnostic.set_spans(vec![id1.span().unwrap(), id2.span().unwrap()]);
        diagnostic.set_message(format!(
          "Same `id: {}` assign to multiple objects, id must be unique.",
          id1
        ));
      }
      DeclareError::CircleDepends(path) => {
        let (msg, spans, note_spans) = path_info(path);
        let msg = format!(
          "There is a directly circle depends exist, this will cause infinite loop: {}",
          msg
        );
        diagnostic.set_spans(spans);
        diagnostic.set_message(msg);
        let note_msg = "You should manual watch to break circle, subscribe \
          only if the value really changed.\n \
          `let_watch!(...).distinct_until_changed().subscribe(...)`
          ";
        diagnostic = diagnostic.span_note(note_spans, note_msg);
      }
      DeclareError::TransitionByConflict(span) => {
        diagnostic.set_spans(*span);
        diagnostic.set_message("field conflict with `by`, To config transition property.");

        diagnostic = diagnostic.span_help(
          *span,
          "When you use `by` field provide a whole `Transition`\
          obj, you can not config other field of `Transition`",
        );
      }
      DeclareError::WatchNothing(span) => {
        diagnostic.set_spans(*span);
        diagnostic.set_message("try to watch a expression without any stateful target.");
      }
      DeclareError::PropInvalidTarget(span) => {
        *tokens = syn::Error::new(*span, "is not a stateful target.").into_compile_error();
      }
      DeclareError::LetWatchWrongPlace(span) => {
        diagnostic.set_spans(*span);
        diagnostic.set_message(
          "`let_watch` only allow start as a statement to help auto\
          unsubscribe a subscribed stream when the root of `widget!` dropped.",
        );
      }
      DeclareError::SynErr(err) => err.clone().into_compile_error().to_tokens(tokens),
    };

    diagnostic.emit();
  }
}

// return a tuple compose by the string display of path, the path follow spans
// and the spans of where `#[skip_nc]` can be added.
fn path_info(path: &[CircleUsedPath]) -> (String, Vec<Span>, Vec<Span>) {
  let msg = path
    .iter()
    .map(|info| {
      if let Some(m) = info.member.as_ref() {
        format!("{}.{} ~> {} ", info.obj, m, info.used_widget)
      } else {
        format!("{} ~> {} ", info.obj, info.used_widget)
      }
    })
    .collect::<Vec<_>>()
    .join(", ");

  let spans = path.iter().fold(vec![], |mut res, info| {
    res.push(info.obj.span().unwrap());
    if let Some(m) = info.member.as_ref() {
      res.push(m.span().unwrap());
    }

    res.push(info.used_widget.span().unwrap());
    let t_spans = info.used_info.spans.iter().map(|s| s.unwrap());
    res.extend(t_spans);
    res
  });

  let note_spans = path
    .iter()
    .map(|info| {
      if let Some(m) = info.member.as_ref() {
        m.span().unwrap()
      } else {
        info
          .used_info
          .spans
          .iter()
          .fold(info.obj.span(), |s1, s2| s2.join(s1).unwrap())
          .unwrap()
      }
    })
    .collect::<Vec<_>>();

  (msg, spans, note_spans)
}

impl DeclareWarning {
  pub fn emit_warning(&self) {
    let mut d = Diagnostic::new(Level::Warning, "");
    match self {
      DeclareWarning::UnusedName(span) => {
        d.set_spans(*span);
        d.set_message("assigned id but not be used in anywhere.");
        d = d.span_help(*span, "Remove this line.");
      }
      DeclareWarning::DefObjWithoutId(span) => {
        d.set_spans(*span);
        d.set_message("Define an object without id.");
        d = d.help("Try to assign an `id` for it.");
      }
    };
    d.emit();
  }
}
