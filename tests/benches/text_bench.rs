use criterion::{Criterion, criterion_group, criterion_main};
use ribir_painter::{shaper::*, *};

fn shape_1k(c: &mut Criterion) {
  let mut shaper = TextShaper::new(<_>::default());
  shaper.font_db().borrow_mut().load_system_fonts();

  let ids = shaper
    .font_db()
    .borrow_mut()
    .select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });

  c.bench_function("shape_1k", |b| {
    b.iter(|| {
      // clean cache
      shaper.end_frame();
      shaper.end_frame();

      let str = include_str!("../../LICENSE").into();
      shaper.shape_text(&str, &ids, TextDirection::LeftToRight)
    })
  });
}

criterion_group!(text_benches, shape_1k);
criterion_main!(text_benches);
