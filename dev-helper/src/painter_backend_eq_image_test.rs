use std::time::{SystemTime, UNIX_EPOCH};

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
  ($painter_fn: ident) => {
    paste::paste! {
      #[test]
      fn [<wgpu_ $painter_fn>]() {
        let mut painter = $painter_fn();
        let viewport = painter.paint_bounds().to_i32().cast_unit();
        let img = wgpu_render_commands(painter.finish(), viewport, Color::TRANSPARENT);
        let name = format!("{}_wgpu", std::stringify!($painter_fn));
        let file_path = test_case_name!(name, "png");
        assert_texture_eq_png(img, &file_path);
      }
    }
  };
}

#[macro_export]
macro_rules! test_case_name {
  ($name: ident, $format: literal) => {{
    let mut path_buffer = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let module_path = std::module_path!();
    let path = module_path.replace("::", "/");
    path_buffer.push(&format!("test_cases/{path}/{}.{}", $name, $format));

    path_buffer.into_boxed_path()
  }};
}

#[track_caller]
pub fn assert_texture_eq_png(img: PixelImage, file_path: &std::path::Path) {
  let overwrite = std::ffi::OsStr::new("overwrite");
  if std::env::var_os("RIBIR_IMG_TEST").map_or(false, |var| var == overwrite) {
    let folders = file_path.parent().unwrap();
    std::fs::create_dir_all(folders).unwrap();
    let mut file = std::fs::File::create(file_path).unwrap();
    img.write_as_png(&mut file).unwrap();
  } else {
    let mut f = std::fs::File::open(file_path).unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut bytes).unwrap();
    let expected = PixelImage::from_png(&bytes);

    assert_eq!(img.pixel_bytes().len(), expected.pixel_bytes().len());
    assert_eq!(img.color_format(), ColorFormat::Rgba8);
    assert_eq!(expected.color_format(), ColorFormat::Rgba8);

    let dssim = dssim_core::Dssim::new();
    let rgba_img = unsafe {
      let ptr = img.pixel_bytes().as_ptr() as *const _;
      std::slice::from_raw_parts(ptr, img.pixel_bytes().len() / 4)
    };
    let dissim_mig = dssim
      .create_image_rgba(rgba_img, img.width() as usize, img.height() as usize)
      .unwrap();
    let rgba_expected = unsafe {
      let ptr = expected.pixel_bytes().as_ptr() as *const _;
      std::slice::from_raw_parts(ptr, expected.pixel_bytes().len() / 4)
    };
    let expected = dssim
      .create_image_rgba(
        rgba_expected,
        expected.width() as usize,
        expected.height() as usize,
      )
      .unwrap();

    const TOLERANCE: f64 = 0.000002;
    let (v, _) = dssim.compare(&expected, dissim_mig);
    let v: f64 = v.into();

    let mut tmp_file = std::env::temp_dir();

    if TOLERANCE <= v {
      let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
      tmp_file.push(format!(
        "{:?}_{}",
        dur,
        file_path.file_name().and_then(|f| f.to_str()).unwrap()
      ));
      let mut file = std::fs::File::create(tmp_file.clone()).unwrap();
      img.write_as_png(&mut file).unwrap();
    }
    assert!(
      v < TOLERANCE,
      "`{v:}` over image test tolerance {TOLERANCE}, new image save at {tmp_file:?}"
    );
  }
}

/// Render painter by wgpu backend, and return the image.
pub fn wgpu_render_commands(
  commands: Vec<ribir_painter::PaintCommand>,
  viewport: ribir_geom::DeviceRect,
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

  backend.begin_frame();
  backend.draw_commands(rect, commands, surface, &mut texture);
  let img = texture.copy_as_image(&rect, backend.get_impl_mut());
  backend.end_frame();
  block_on(img).unwrap()
}
