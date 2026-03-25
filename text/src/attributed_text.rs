use ribir_algo::CowArc;

use crate::{
  paragraph::{TextRange, TextSpan},
  style::SpanStyle,
};

/// A read-only rich text value made of one logical string plus styled byte
/// ranges.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributedText<Brush> {
  pub text: CowArc<str>,
  pub spans: Box<[TextSpan<Brush>]>,
}

impl<Brush> Default for AttributedText<Brush> {
  fn default() -> Self { Self::plain("") }
}

impl<Brush> AttributedText<Brush> {
  #[inline]
  pub fn plain(text: impl Into<CowArc<str>>) -> Self {
    Self { text: text.into(), spans: Vec::new().into_boxed_slice() }
  }

  #[inline]
  pub fn styled(text: impl Into<CowArc<str>>, style: SpanStyle<Brush>) -> Self {
    let text = text.into();
    let spans = if text.is_empty() {
      Default::default()
    } else {
      vec![TextSpan { range: TextRange::new(0, text.len()), style }].into_boxed_slice()
    };
    Self { text, spans }
  }

  #[inline]
  pub fn from_parts(
    text: impl Into<CowArc<str>>, spans: impl Into<Box<[TextSpan<Brush>]>>,
  ) -> Self {
    Self { text: text.into(), spans: spans.into() }
  }

  #[inline]
  pub fn builder() -> AttributedTextBuilder<Brush> { AttributedTextBuilder::default() }

  #[inline]
  pub fn len_bytes(&self) -> usize { self.text.len() }

  #[inline]
  pub fn is_empty(&self) -> bool { self.text.is_empty() }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributedTextBuilder<Brush> {
  text: String,
  spans: Vec<TextSpan<Brush>>,
}

impl<Brush> Default for AttributedTextBuilder<Brush> {
  fn default() -> Self { Self { text: String::new(), spans: Vec::new() } }
}

impl<Brush> AttributedTextBuilder<Brush> {
  #[inline]
  pub fn push_text(mut self, text: impl AsRef<str>) -> Self {
    self.write_text(text);
    self
  }

  #[inline]
  pub fn write_text(&mut self, text: impl AsRef<str>) -> &mut Self {
    self.text.push_str(text.as_ref());
    self
  }

  #[inline]
  pub fn push_styled_text(mut self, text: impl AsRef<str>, style: SpanStyle<Brush>) -> Self {
    self.write_styled_text(text, style);
    self
  }

  #[inline]
  pub fn write_styled_text(&mut self, text: impl AsRef<str>, style: SpanStyle<Brush>) -> &mut Self {
    let text = text.as_ref();
    if text.is_empty() {
      return self;
    }

    let start = self.text.len();
    self.text.push_str(text);
    let end = self.text.len();
    self
      .spans
      .push(TextSpan { range: TextRange::new(start, end), style });
    self
  }

  #[inline]
  pub fn append(mut self, text: AttributedText<Brush>) -> Self {
    self.write_attributed_text(text);
    self
  }

  pub fn write_attributed_text(&mut self, text: AttributedText<Brush>) -> &mut Self {
    let AttributedText { text, spans } = text;
    let offset = self.text.len();
    self.text.push_str(text.as_ref());
    self
      .spans
      .extend(spans.into_vec().into_iter().map(|mut span| {
        span.range.start.0 += offset;
        span.range.end.0 += offset;
        span
      }));
    self
  }

  #[inline]
  pub fn build(self) -> AttributedText<Brush> {
    AttributedText { text: self.text.into(), spans: self.spans.into_boxed_slice() }
  }
}

impl<Brush> From<AttributedTextBuilder<Brush>> for AttributedText<Brush> {
  #[inline]
  fn from(value: AttributedTextBuilder<Brush>) -> Self { value.build() }
}

impl<Brush> From<CowArc<str>> for AttributedText<Brush> {
  #[inline]
  fn from(value: CowArc<str>) -> Self { Self::plain(value) }
}

impl<Brush> From<String> for AttributedText<Brush> {
  #[inline]
  fn from(value: String) -> Self { Self::plain(value) }
}

impl<Brush> From<&str> for AttributedText<Brush> {
  #[inline]
  fn from(value: &str) -> Self { Self::plain(value.to_owned()) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builder_shifts_ranges_when_appending_text() {
    let rich = AttributedText::builder()
      .push_text("Hello ")
      .push_styled_text("Ribir", SpanStyle { brush: Some(7_u8), ..Default::default() })
      .append(AttributedText::styled("!", SpanStyle { brush: Some(9_u8), ..Default::default() }))
      .build();

    assert_eq!(&*rich.text, "Hello Ribir!");
    assert_eq!(rich.spans.len(), 2);
    assert_eq!(rich.spans[0].range, TextRange::new(6, 11));
    assert_eq!(rich.spans[1].range, TextRange::new(11, 12));
    assert_eq!(rich.spans[0].style.brush, Some(7));
    assert_eq!(rich.spans[1].style.brush, Some(9));
  }

  #[test]
  fn from_parts_keeps_text_and_spans() {
    let spans = vec![TextSpan {
      range: TextRange::new(0, 1),
      style: SpanStyle { font_size: Some(18.), brush: Some(3_u8), ..Default::default() },
    }]
    .into_boxed_slice();

    let rich = AttributedText::from_parts("A", spans.clone());

    assert_eq!(&*rich.text, "A");
    assert_eq!(rich.spans, spans);
  }
}
