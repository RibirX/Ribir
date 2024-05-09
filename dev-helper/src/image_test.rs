use ribir_painter::{image::ColorFormat, PixelImage};

/// This macro generates image tests for the painter with every backend. Accept
/// a function returning a painter. The generated test name is the function name
/// composed a prefix(the backend name). The test will check if the backend
/// renders the painter result to generate the same content as the image file.
///
/// The image file is read from the `test_cases` folder in the workspace root,
/// and its path relative to the `test_cases` is `{module path}\{backend
/// name}\{function name}.{fmt}`:
///
/// - the `{module path}` is where the generated test is placed.
/// - the `{backend name}` is the painter-backend name like `wgpu`.
/// - the `{function  name}` is the function you pass to the macro.
/// - the `{fmt}` is the file format the backend wants to check.
///
/// You can run the test with `RIBIR_IMG_TEST=overwrite` to overwrite the image
/// file, for example ```
/// RIBIR_IMG_TEST=overwrite cargo test -- smoke
#[macro_export]
macro_rules! painter_backend_eq_image_test {
  ($painter_fn:ident $(, comparison = $comparison:expr)?) => {
    paste::paste! {
      #[test]
      fn [<wgpu_ $painter_fn>]() {
        let mut painter = $painter_fn();
        let viewport = painter.viewport().to_i32().cast_unit();
        let img = wgpu_render_commands(painter.finish(), viewport, Color::TRANSPARENT);
        let name = format!("{}_wgpu", std::stringify!($painter_fn));
        let file_path = test_case_name!(name, "png");
        ImageTest::new(img, &file_path)
          $(.with_comparison($comparison))?
          .test();
      }
    }
  };
}

#[macro_export]
macro_rules! test_case_name {
  ($name:ident, $format:literal) => {{
    let mut path_buffer = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let module_path = std::module_path!();
    let path = module_path.replace("::", "/");
    path_buffer.push(&format!("test_cases/{path}/{}.{}", $name, $format));

    path_buffer
  }};
}

pub struct ImageTest<'a> {
  test_img: PixelImage,
  ref_path: &'a std::path::Path,
  comparison: f32,
}

impl<'a> ImageTest<'a> {
  pub fn new(test_img: PixelImage, ref_path: &'a std::path::Path) -> Self {
    Self { test_img, ref_path, comparison: 0.000001 }
  }

  pub fn with_comparison(mut self, comparison: f32) -> Self {
    self.comparison = comparison;
    self
  }

  #[track_caller]
  pub fn test(self) {
    let Self { test_img, ref_path, comparison } = self;

    use std::fs::File;

    let overwrite = std::ffi::OsStr::new("overwrite");
    let dir = ref_path.parent().unwrap();
    let stem = ref_path.file_stem().unwrap().to_str().unwrap();
    if std::env::var_os("RIBIR_IMG_TEST").map_or(false, |var| var == overwrite) {
      std::fs::create_dir_all(dir).unwrap();
      let mut file = File::create(ref_path).unwrap();
      test_img.write_as_png(&mut file).unwrap();
    } else {
      let mut f = File::open(ref_path).unwrap();
      let mut bytes = Vec::new();
      std::io::Read::read_to_end(&mut f, &mut bytes).unwrap();
      let ref_img = PixelImage::from_png(&bytes);

      assert_eq!(test_img.pixel_bytes().len(), ref_img.pixel_bytes().len());
      assert_eq!(test_img.color_format(), ColorFormat::Rgba8);
      assert_eq!(ref_img.color_format(), ColorFormat::Rgba8);

      let test_filp = nv_flip::FlipImageRgb8::with_data(
        test_img.width(),
        test_img.height(),
        &Self::rgba_2_rgb_pixels(test_img.pixel_bytes()),
      );

      let ref_flip = nv_flip::FlipImageRgb8::with_data(
        ref_img.width(),
        ref_img.height(),
        &Self::rgba_2_rgb_pixels(ref_img.pixel_bytes()),
      );
      let error_map = nv_flip::flip(ref_flip, test_filp, nv_flip::DEFAULT_PIXELS_PER_DEGREE);
      let visualized = error_map.apply_color_lut(&nv_flip::magma_lut());

      let pool = nv_flip::FlipPool::from_image(&error_map);
      let mean = pool.mean();
      let diff_path = dir.join(format!("{stem}_diff.png"));
      let actual_path = dir.join(format!("{stem}_actual.png"));
      if mean > f32::EPSILON {
        // write the actual image to the same folder
        test_img
          .write_as_png(&mut File::create(&actual_path).unwrap())
          .unwrap();

        // write the diff image to the same folder
        image::RgbImage::from_raw(visualized.width(), visualized.height(), visualized.to_vec())
          .unwrap()
          .save(&diff_path)
          .unwrap();
      }

      assert!(
        mean < comparison,
        "Image test failed. Expected Mean({mean}) to be less than {comparison}. The actual image \
         and difference image have been saved next to the expected image.
      Expected image location: {ref_path:?}
      Actual image location: {actual_path:?}
      Difference file location: {diff_path:?}"
      );
    }
  }

  fn rgba_2_rgb_pixels(data: &[u8]) -> Vec<u8> {
    let mut res = Vec::with_capacity(data.len() / 4 * 3);
    for chunk in data.chunks(4) {
      res.extend_from_slice(&chunk[..3]);
    }
    res
  }
}

#[track_caller]
pub fn assert_texture_eq_png(test_img: PixelImage, ref_path: &std::path::Path) {
  ImageTest::new(test_img, ref_path).test();
}

/// Render painter by wgpu backend, and return the image.
pub fn wgpu_render_commands(
  commands: Vec<ribir_painter::PaintCommand>, viewport: ribir_geom::DeviceRect,
  surface: ribir_painter::Color,
) -> PixelImage {
  use futures::executor::block_on;
  use ribir_geom::{DeviceRect, DeviceSize};
  use ribir_gpu::{GPUBackend, GPUBackendImpl, Texture};
  use ribir_painter::{AntiAliasing, PainterBackend};

  let mut gpu_impl = block_on(ribir_gpu::WgpuImpl::headless());

  let rect = DeviceRect::from_size(DeviceSize::new(viewport.max_x() + 2, viewport.max_y() + 2));
  let mut texture = gpu_impl.new_texture(rect.size, AntiAliasing::None, ColorFormat::Rgba8);
  let mut backend = GPUBackend::new(gpu_impl, AntiAliasing::None);

  backend.begin_frame(surface);
  backend.draw_commands(rect, commands, &mut texture);
  let img = texture.copy_as_image(&rect, backend.get_impl_mut());
  backend.end_frame();
  block_on(img).unwrap()
}
