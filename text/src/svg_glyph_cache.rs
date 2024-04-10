use std::{borrow::Cow, collections::BTreeMap, io::prelude::*, ops::RangeInclusive};

use ahash::{HashMap, HashSet};
use log::warn;
use quick_xml::{
  events::{attributes::Attribute, BytesStart, Event},
  name::QName,
  reader::Reader,
};
use ribir_painter::Svg;
use rustybuzz::ttf_parser::GlyphId;

#[derive(Default)]
pub struct SvgGlyphCache {
  svg_docs: SvgDocumentCache,
  svg_glyphs: HashMap<GlyphId, Option<Svg>>,
}

impl SvgGlyphCache {
  pub fn svg_or_insert(&mut self, glyph_id: GlyphId, rb_face: &rustybuzz::Face) -> &Option<Svg> {
    let SvgGlyphCache { svg_docs, svg_glyphs } = self;
    svg_glyphs.entry(glyph_id).or_insert_with(|| {
      if let Some(doc) = svg_docs.get(glyph_id) {
        doc.glyph_svg(glyph_id, rb_face)
      } else {
        rb_face.glyph_svg_image(glyph_id).and_then(|doc| {
          let doc = SvgDocument::new(doc.glyphs_range(), doc.data);
          let svg = doc.glyph_svg(glyph_id, rb_face);
          svg_docs.insert(doc);
          svg
        })
      }
    })
  }
}

#[derive(Default)]
struct SvgDocumentCache {
  docs: BTreeMap<GlyphId, SvgDocument>,
}

impl SvgDocumentCache {
  fn insert(&mut self, doc: SvgDocument) { self.docs.insert(*doc.range.start(), doc); }

  fn get(&self, glyph_id: GlyphId) -> Option<&SvgDocument> {
    // use btreemap.lower_bound is better, but it's unstable now
    let its = self.docs.range(..=glyph_id);
    its
      .last()
      .and_then(|(_, doc)| doc.range.contains(&glyph_id).then_some(doc))
  }
}

struct SvgDocument {
  range: RangeInclusive<GlyphId>,
  elems: HashMap<String, String>,
}

impl SvgDocument {
  fn new(range: RangeInclusive<GlyphId>, content: &[u8]) -> Self {
    let elems = Self::parse(content).unwrap_or_default();

    Self { range, elems }
  }

  fn glyph_svg(&self, glyph: GlyphId, face: &rustybuzz::Face) -> Option<Svg> {
    let key = format!("glyph{}", glyph.0);
    if !self.elems.contains_key(&key) {
      return None;
    }

    let mut all_links = HashSet::default();
    let mut elems = vec![key.clone()];

    while let Some(curr) = elems.pop() {
      if let Some(content) = self.elems.get(&curr) {
        elems.extend(Self::collect_link(content, &mut all_links));
      }
    }

    let units_per_em = face.units_per_em();
    let ascender = face.ascender() as i32;
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
    writer
      .write_all(self.elems.get(&key).unwrap().as_bytes())
      .ok()?;
    writer.write_all("</svg>".as_bytes()).ok()?;

    std::str::from_utf8(&writer.into_inner())
      .ok()
      .and_then(|str| Svg::parse_from_bytes(str.as_bytes()).ok())
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
        Ok(Event::Eof) => break, // exits the loop when reaching end of file
        Err(e) => {
          warn!("Error at position {}: {:?}", reader.buffer_position(), e);
          return None;
        }

        _ => (), // There are several other `Event`s we do not consider here
      }
    }
    Some(elems)
  }

  fn collect_named_obj(
    reader: &mut Reader<&[u8]>, elems: &mut HashMap<String, String>, source: &str, e: &BytesStart,
    has_children: bool,
  ) {
    if let Some(id) = e
      .attributes()
      .find(|a| {
        a.as_ref()
          .map_or(false, |a| a.key == QName(b"id"))
      })
      .map(|a| a.unwrap().value)
    {
      unsafe {
        let content = Self::extra_elem(reader, e, source, has_children);
        elems.insert(std::str::from_utf8_unchecked(&id).to_string(), content);
      }
    };
  }

  unsafe fn extra_elem(
    reader: &mut Reader<&[u8]>, e: &BytesStart, source: &str, has_children: bool,
  ) -> String {
    let content = if has_children {
      let mut buf = Vec::new();
      let rg = reader
        .read_to_end_into(e.name().to_owned(), &mut buf)
        .unwrap();
      &source[rg.start..rg.end]
    } else {
      ""
    };

    let name = e.name();
    let name = reader.decoder().decode(name.as_ref()).unwrap();

    format!("<{}>{}</{}>", std::str::from_utf8_unchecked(e), content, name)
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
        Ok(Event::Eof) => break, // exits the loop when reaching end of file
        Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),

        _ => (), // There are several other `Event`s we do not consider here
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
    let attributes = elem.attributes();

    attributes.for_each(|attr| {
      let attr = attr.unwrap();
      if let Some(link) =
        Self::extra_link_from_href(&attr).or_else(|| Self::extra_link_from_iri_func(attr.value))
      {
        if all_links.contains(&link) {
          return;
        }
        all_links.insert(link.clone());
        new_links.push(link);
      }
    });
  }
}

#[cfg(test)]
mod tests {
  use rustybuzz::ttf_parser::GlyphId;

  use super::{SvgDocument, SvgDocumentCache};
  use crate::font_db::FontDB;

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
    let doc = super::SvgDocument::new(GlyphId(2428)..=GlyphId(2428), content.as_bytes());
    let mut db = FontDB::default();
    let dummy_face = db
      .face_data_or_insert(db.default_fonts()[0])
      .unwrap();
    assert_eq!(doc.elems.len(), 4);
    assert!(
      doc
        .glyph_svg(GlyphId(2428), dummy_face.as_rb_face())
        .is_some()
    );
    assert!(
      doc
        .glyph_svg(GlyphId(0), dummy_face.as_rb_face())
        .is_none()
    );
  }

  #[test]
  fn test_svg_document_cache() {
    let mut cache = SvgDocumentCache::default();
    cache.insert(SvgDocument::new(GlyphId(0)..=GlyphId(10), "".as_bytes()));
    cache.insert(SvgDocument::new(GlyphId(11)..=GlyphId(20), "".as_bytes()));
    cache.insert(SvgDocument::new(GlyphId(31)..=GlyphId(40), "".as_bytes()));

    assert_eq!(
      Some(GlyphId(11)),
      cache
        .get(GlyphId(11))
        .map(|doc| *doc.range.start())
    );

    assert_eq!(
      Some(GlyphId(31)),
      cache
        .get(GlyphId(40))
        .map(|doc| *doc.range.start())
    );

    assert_eq!(
      None,
      cache
        .get(GlyphId(21))
        .map(|doc| *doc.range.start())
    );
  }
}
