use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use ribir::{
  core::{reset_test_env, test_helper::*},
  prelude::*,
};

fn bench_example<K: ?Sized>(b: &mut Bencher, f: impl RInto<GenWidget, K>) {
  let _ = AppCtx::shared();
  let f: GenWidget = f.r_into();
  b.iter(|| {
    let wnd = TestWindow::from_widget(f.clone());
    wnd.draw_frame();
    AppCtx::remove_wnd(wnd.id())
  })
}

fn bench_example_with_data<K, D: 'static, W: IntoWidget<'static, K>>(
  b: &mut Bencher, data: D, f: impl Fn(&'static D) -> W + 'static,
) {
  let _ = AppCtx::shared();
  let widget_builder = move || f(unsafe { &*(&data as *const _) }).into_widget();
  bench_example(b, widget_builder);
}

fn examples(c: &mut Criterion) {
  reset_test_env!();

  let mut g = c.benchmark_group("Examples");

  g.bench_function("todos", |b| bench_example(b, todos::todos));
  g.bench_function("counter", |b| bench_example_with_data(b, Stateful::new(0), counter::counter));

  g.bench_function("messages", |b| bench_example(b, messages::messages));
  g.bench_function("storybook", |b| bench_example(b, storybook::storybook));
  g.bench_function("wordle_game", |b| bench_example(b, wordle_game::wordle_game));
}

criterion_group!(example_benches, examples);
criterion_main!(example_benches);
