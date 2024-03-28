use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use ribir::core::{animation::easing::*, prelude::*, reset_test_env, test_helper::*};

#[derive(Clone, Debug)]
pub struct Embed {
  pub width: usize,
  pub depth: usize,
}

impl Compose for Embed {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      let recursive_child: Widget = if $this.depth > 1 {
        let width = $this.width;
        let depth = $this.depth - 1;
        Embed { width, depth }.build(ctx!())
      } else {
        MockBox { size: Size::new(10., 10.) }.build(ctx!())
      };
      let multi = pipe!{
        (0..$this.width - 1).map(|_| MockBox { size: Size::new(10., 10.)})
      };
      MockMulti
        .with_child(multi, ctx!())
        .with_child(recursive_child, ctx!())

    }
  }
}

#[derive(Clone, Debug)]
pub struct Recursive {
  pub width: usize,
  pub depth: usize,
}

impl Compose for Recursive {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @MockMulti {
        @{
          pipe!(($this.width, $this.depth))
            .map(move |(width, depth)| {
              (0..width).map(move |_| -> Widget {
                if depth > 1 {
                  Recursive { width, depth: depth - 1 }.build(ctx!())
                } else {
                  MockBox { size: Size::new(10., 10.)}.build(ctx!())
                }
              })
            })
        }
       }
    }
  }
}

fn bench_widget_inflate(b: &mut Bencher, w: impl Compose + Clone + 'static) {
  let mut wnd = TestWindow::new(fn_widget!(Void));
  b.iter(|| {
    let w = w.clone();
    wnd.set_content_widget(fn_widget! { w });
    wnd.draw_frame();
  });
  AppCtx::remove_wnd(wnd.id());
}

fn bench_widget_inflate_pow(width: usize, depth: usize, b: &mut Bencher) {
  bench_widget_inflate(b, Recursive { width, depth });
}

fn bench_widget_inflate_x(width: usize, depth: usize, b: &mut Bencher) {
  bench_widget_inflate(b, Embed { width, depth });
}

fn bench_widget_repair(b: &mut Bencher, w: State<impl Compose + 'static>) {
  let trigger = w.clone_writer();
  let mut wnd = TestWindow::new(fn_widget!(w));
  let id = wnd.id();
  b.iter(move || {
    {
      let _ = trigger.write();
    }
    wnd.draw_frame();
  });
  AppCtx::remove_wnd(id);
}

fn bench_recursive_repair_pow(width: usize, depth: usize, b: &mut Bencher) {
  bench_widget_repair(b, State::value(Recursive { width, depth }))
}

fn bench_recursive_repair_x(width: usize, depth: usize, b: &mut Bencher) {
  bench_widget_repair(b, State::value(Embed { width, depth }))
}

fn tree_build_regen(c: &mut Criterion) {
  reset_test_env!();
  let mut group = c.benchmark_group("Widget Tree");
  group.sample_size(80);

  group.bench_function("new_100_x_5", |b| bench_widget_inflate_x(100, 5, b));
  group.bench_function("regen_100_x_5", |b| bench_recursive_repair_x(100, 5, b));
  group.bench_function("new_50_x_50", |b| bench_widget_inflate_x(50, 50, b));
  group.bench_function("regen_50_x_50", |b| bench_recursive_repair_x(50, 50, b));
  group.bench_function("new_10_x_1000", |b| bench_widget_inflate_x(10, 1000, b));
  group.bench_function("regen_10_x_1000", |b| bench_recursive_repair_x(10, 1000, b));
  group.bench_function("new_50_pow_2", |b| bench_widget_inflate_pow(50, 2, b));
  group.bench_function("regen_50_pow_2", |b| bench_recursive_repair_pow(50, 2, b));
  group.bench_function("new_100_pow_2", |b| bench_widget_inflate_pow(100, 2, b));
  group.bench_function("regen_100_pow_2", |b| bench_recursive_repair_pow(100, 2, b));
  group.bench_function("new_10_pow_4", |b| bench_widget_inflate_pow(10, 4, b));
  group.bench_function("regen_10_pow_4", |b| bench_recursive_repair_pow(10, 4, b));
  group.bench_function("new_10_pow_5", |b| bench_widget_inflate_pow(10, 5, b));
  group.bench_function("regen_10_pow_5", |b| bench_recursive_repair_pow(10, 5, b));
}

fn fn_bench(c: &mut Criterion) {
  reset_test_env!();

  c.bench_function("lerp_color", |b| {
    b.iter(|| {
      let sum: u32 = (0..100)
        .map(|i| Lerp::lerp(&Color::from_u32(i), &Color::from_u32(0xff_ff_ff), 0.3).into_u32())
        .sum();
      sum
    })
  });

  c.bench_function("curve_bezir", |b| {
    b.iter(|| {
      let sum: f32 = (0..1000)
        .map(|i| CubicBezierEasing::new(0.3, 0.7, 0.4, 0.3).easing(i as f32 / 1001.))
        .sum();
      sum
    })
  });
}

criterion_group!(core, fn_bench, tree_build_regen);
criterion_main!(core);
