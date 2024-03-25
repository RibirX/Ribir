use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use ribir::core::{reset_test_env, test_helper::*};
use ribir::prelude::*;

fn bench_example<F: Fn() -> R, R: WidgetBuilder>(b: &mut Bencher, f: F) {
  let _ = AppCtx::shared();
  b.iter(|| {
    let mut wnd = TestWindow::new(f());
    wnd.draw_frame();
    AppCtx::remove_wnd(wnd.id())
  })
}

fn examples(c: &mut Criterion) {
  reset_test_env!();

  let mut g = c.benchmark_group("Examples");

  g.bench_function("todos", |b| bench_example(b, todos::todos));
  g.bench_function("counter", |b| bench_example(b, counter::counter));
  g.bench_function("messages", |b| bench_example(b, messages::messages));
  g.bench_function("storybook", |b| bench_example(b, storybook::storybook));
  g.bench_function("wordle_game", |b| {
    bench_example(b, wordle_game::wordle_game)
  });
}

criterion_group!(example_benches, examples);
criterion_main!(example_benches);
