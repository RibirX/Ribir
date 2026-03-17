use std::{borrow::Cow, io::prelude::*};

use ahash::{HashMap, HashSet};
use quick_xml::{
  events::{BytesStart, Event, attributes::Attribute},
  name::QName,
  reader::Reader,
};
use swash::FontRef;
use tracing::warn;

use crate::paint::GlyphId;

/// Extract SVG glyph from OpenType font.
///
/// `swash` doesn't expose SVG glyph documents directly, so we extract them
/// through `ttf-parser`, which is already in the dependency tree.
pub(crate) fn extract_svg_glyph(
  glyph_id: GlyphId, font: &FontRef, face_index: u32,
) -> Option<String> {
  let face = ttf_parser::Face::parse(font.data, face_index).ok()?;
  let doc = face.glyph_svg_image(ttf_parser::GlyphId(glyph_id.0))?;
  let svg_doc =
    SvgDocument::new(doc.glyphs_range().start().0..=doc.glyphs_range().end().0, doc.data);
  svg_doc.glyph_svg(glyph_id, font)
}

struct SvgDocument {
  elems: HashMap<String, String>,
}

impl SvgDocument {
  fn new(_range: std::ops::RangeInclusive<u16>, content: &[u8]) -> Self {
    let elems = Self::parse(content).unwrap_or_default();
    Self { elems }
  }

  fn glyph_svg(&self, glyph: GlyphId, font: &FontRef) -> Option<String> {
    let key = format!("glyph{}", glyph.0);
    let mut all_links = HashSet::default();
    let mut elems = vec![key.clone()];

    while let Some(curr) = elems.pop() {
      if let Some(content) = self.elems.get(&curr) {
        elems.extend(Self::collect_link(content, &mut all_links));
      }
    }

    let root = self.elems.get(&key)?;

    let units_per_em = font.metrics(&[]).units_per_em;
    let metrics = font.metrics(&[]);
    let ascender = metrics.ascent as i32;
    let mut writer = std::io::Cursor::new(Vec::new());

    writer
      .write_all(
        format!(
          "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
           version=\"1.1\" width=\"{}\" height=\"{}\" viewBox=\"{},{},{},{}\">",
          units_per_em, units_per_em, 0, -ascender, units_per_em, units_per_em
        )
        .as_bytes(),
      )
      .ok()?;
    writer.write_all("<defs>".as_bytes()).ok()?;
    for link in all_links {
      if let Some(content) = self.elems.get(&link) {
        writer.write_all(content.as_bytes()).ok()?;
      }
    }
    writer.write_all("</defs>".as_bytes()).ok()?;
    writer.write_all(root.as_bytes()).ok()?;
    writer.write_all("</svg>".as_bytes()).ok()?;

    String::from_utf8(writer.into_inner()).ok()
  }

  fn parse(data: &[u8]) -> Option<HashMap<String, String>> {
    let content = std::str::from_utf8(data).ok()?;
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    let mut elems = HashMap::default();
    loop {
      match reader.read_event_into(&mut buf) {
        Ok(ref e @ Event::Start(ref tag)) | Ok(ref e @ Event::Empty(ref tag)) => {
          if tag.name() != QName(b"defs") {
            let has_child = matches!(e, Event::Start(_));
            Self::collect_named_obj(&mut reader, &mut elems, content, tag, has_child);
          }
        }
        Ok(Event::Eof) => break,
        Err(e) => {
          warn!("Error at position {}: {:?}", reader.buffer_position(), e);
          return None;
        }
        _ => (),
      }
    }
    Some(elems)
  }

  fn collect_named_obj(
    reader: &mut Reader<&[u8]>, elems: &mut HashMap<String, String>, source: &str, e: &BytesStart,
    has_children: bool,
  ) {
    let Some(id) = e.attributes().find_map(|attr| {
      let attr = attr.ok()?;
      (attr.key == QName(b"id")).then(|| String::from_utf8_lossy(&attr.value).into_owned())
    }) else {
      return;
    };

    if let Some(content) = Self::extract_elem(reader, e, source, has_children) {
      elems.insert(id, content);
    }
  }

  fn extract_elem(
    reader: &mut Reader<&[u8]>, e: &BytesStart, source: &str, has_children: bool,
  ) -> Option<String> {
    let content = if has_children {
      let mut buf = Vec::new();
      let rg = reader
        .read_to_end_into(e.name().to_owned(), &mut buf)
        .ok()?;
      &source[rg.start as usize..rg.end as usize]
    } else {
      ""
    };

    let name = e.name();
    let name = reader.decoder().decode(name.as_ref()).ok()?;
    let start = std::str::from_utf8(e.as_ref()).ok()?;

    Some(format!("<{}>{}</{}>", start, content, name))
  }

  fn collect_link(content: &str, all_links: &mut HashSet<String>) -> Vec<String> {
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    let mut new_links = Vec::new();
    loop {
      match reader.read_event_into(&mut buf) {
        Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
          Self::collect_link_from_attrs(e, all_links, &mut new_links);
        }
        Ok(Event::Eof) => break,
        Err(e) => {
          warn!("Error at position {}: {:?}", reader.buffer_position(), e);
          break;
        }
        _ => (),
      }
    }
    new_links
  }

  #[inline]
  fn extra_link_from_iri_func(val: Cow<'_, [u8]>) -> Option<String> {
    let val: &str = std::str::from_utf8(&val)
      .unwrap()
      .trim()
      .strip_prefix("url(")?
      .trim_start()
      .strip_prefix('#')?
      .strip_suffix(')')?;
    Some(val.to_string())
  }

  #[inline]
  fn extra_link_from_href(attr: &Attribute) -> Option<String> {
    if attr.key == QName(b"xlink:href") || attr.key == QName(b"href") {
      let href = std::str::from_utf8(&attr.value).unwrap();
      return Some(href.trim().strip_prefix('#')?.to_string());
    }
    None
  }

  fn collect_link_from_attrs(
    elem: &BytesStart, all_links: &mut HashSet<String>, new_links: &mut Vec<String>,
  ) {
    for attr in elem.attributes().flatten() {
      let Some(link) =
        Self::extra_link_from_href(&attr).or_else(|| Self::extra_link_from_iri_func(attr.value))
      else {
        continue;
      };

      if all_links.insert(link.clone()) {
        new_links.push(link);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::SvgDocument;
  use crate::paint::GlyphId;

  #[test]
  fn test_svg_document() {
    let content = r##"
        <svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1">
          <defs>
            <path
              d="M262,-672 Q222,-610 216,-563 Q210,-516 237,-500 Q250,-493 262,-501 Q274,-509 284.5,-525 Q295,-541 303.5,-558.5 Q312,-576 319,-586 Q399,-705 535,-749 Q545,-753 556.5,-758 Q568,-763 573,-773 Q579,-785 572.5,-794.5 Q566,-804 554,-808 Q540,-814 522.5,-813.5 Q505,-813 488,-810 Q417,-798 355,-759.5 Q293,-721 262,-672 Z"
              id="u1F250.2"></path>
            <path
              d="M393,25 Q393,-4 372.5,-24 Q352,-44 324,-44 Q296,-44 276,-24 Q256,-4 256,25 Q256,53 276,73 Q296,93 324,93 Q352,93 372.5,73 Q393,53 393,25 Z"
              id="u1F69E.17"></path>
            <radialGradient id="g799" cx="638" cy="380" r="508" gradientUnits="userSpaceOnUse"
              gradientTransform="matrix(1 0 0 0.525 0 0)">
              <stop offset="0.598" stop-color="#212121" />
              <stop offset="1" stop-color="#616161" />
            </radialGradient>
          </defs>
          <g id="glyph2428">
            <use xlink:href="#u1F69E.17" x="-1886.951" y="-548.858"
              transform="matrix(7.674 0 0 7.674 12593.511 3663.078)" fill="#FFCC32" />
          </g>
        </svg>"##;
    let doc = SvgDocument::new(2428..=2428, content.as_bytes());
    let font_bytes = include_bytes!("Lato-Regular.ttf");
    let dummy_font_ref = swash::FontRef::from_index(font_bytes, 0).unwrap();
    assert_eq!(doc.elems.len(), 4);
    assert!(
      doc
        .glyph_svg(GlyphId(2428), &dummy_font_ref)
        .is_some()
    );
    assert!(
      doc
        .glyph_svg(GlyphId(0), &dummy_font_ref)
        .is_none()
    );
  }
}
