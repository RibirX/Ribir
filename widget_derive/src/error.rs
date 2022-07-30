use crate::widget_attr_macro::NameUsedInfo;
use proc_macro::{Diagnostic, Level, Span};
use proc_macro2::TokenStream;

use quote::quote;
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
  CircleInit(Box<[CircleUsedPath]>),
  CircleFollow(Box<[CircleUsedPath]>),
  DataFlowNoDepends(Span),
  KeyDependsOnOther {
    key: Span,
    depends_on: Vec<Span>,
  },
  DependOBuiltinFieldWithIfGuard {
    wrap_name: Ident,
    wrap_def_spans: [Span; 3],
    use_spans: Vec<Span>,
  },
  ExprWidgetInvalidField(Vec<Span>),
  UnsupportedIfGuard {
    name: String,
    span: Span,
  },
}

#[derive(Debug)]
pub enum DeclareWarning<'a> {
  NeedlessSkipNc(Span),
  UnusedName(&'a Ident),
}

pub type Result<T> = std::result::Result<T, DeclareError>;

impl DeclareError {
  pub fn into_compile_error(self) -> TokenStream {
    self.error_emit();
    quote! { unreachable!() }
  }

  pub fn error_emit(self) {
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
        let (msg, spans, note_spans) = path_info(&path);
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
        let (msg, spans, note_spans) = path_info(&path);
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

      DeclareError::DataFlowNoDepends(span) => {
        diagnostic.set_spans(span);
        diagnostic.set_message("Declared a data flow but not depends on any others.");
        diagnostic = diagnostic.help("Try to remove it.");
      }
      DeclareError::KeyDependsOnOther { key, mut depends_on } => {
        depends_on.push(key);
        diagnostic.set_spans(depends_on);
        diagnostic.set_message("The `key` field is not allowed to depend on others.");
      }
      DeclareError::DependOBuiltinFieldWithIfGuard { wrap_def_spans, use_spans, .. } => {
        diagnostic.set_spans(use_spans);
        diagnostic.set_message( "Depends on a widget field which behind `if guard`, its existence depends on the `if guard` result in runtime.");
        diagnostic = diagnostic.span_warning(wrap_def_spans.to_vec(), "field define here.");
      }
      DeclareError::ExprWidgetInvalidField(spans) => {
        diagnostic.set_spans(spans);
        diagnostic.set_message("`ExprWidget` only accept `expr` field.");
      }
      DeclareError::UnsupportedIfGuard { name, span } => {
        diagnostic.set_spans(span);
        diagnostic.set_message(format!("{name}  not support if guard"));
      }
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
        format!("{}.{} ～> {} ", info.obj, m, info.used_widget)
      } else {
        format!("{} ～> {} ", info.obj, info.used_widget)
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

impl<'a> DeclareWarning<'a> {
  pub fn emit_warning(&self) {
    let mut d = Diagnostic::new(Level::Warning, "");
    match self {
      DeclareWarning::NeedlessSkipNc(span) => {
        d.set_spans(*span);
        d.set_message("Unnecessary attribute, because not depends on any others");
        d = d.help("Try to remove it.");
      }
      DeclareWarning::UnusedName(name) => {
        d.set_spans(name.span().unwrap());
        d.set_message(format!("`{name}` does not be used"));
        d = d.span_help(vec![name.span().unwrap()], "Remove this line.");
      }
    };
    d.emit();
  }
}
