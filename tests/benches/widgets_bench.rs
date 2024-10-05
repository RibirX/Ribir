use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use ribir::{core::test_helper::*, prelude::*};

fn widget_bench(b: &mut Bencher, w: GenWidget) {
  let mut wnd = TestWindow::new(w);
  b.iter(|| wnd.draw_frame());
  AppCtx::remove_wnd(wnd.id())
}

fn widgets_bench_one_by_one(c: &mut Criterion) {
  c.bench_function("checkbox", |b| {
    widget_bench(b, fn_widget!(@Checkbox { checked: true, indeterminate: true }).into());
  });
}

criterion_group!(widgets_benches, widgets_bench_one_by_one);
criterion_main!(widgets_benches);
