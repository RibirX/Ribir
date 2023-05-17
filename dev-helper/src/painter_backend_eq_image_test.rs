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
    #[cfg(feature = "wgpu")]
    paste::paste! {
      #[test]
      fn [<wgpu_ $painter_fn>]() {
        let mut painter = $painter_fn();
        let img = wgpu_render_commands(&mut painter);

        let file_path = test_case_name!("wgpu", std::stringify!($painter_fn), "png");
        assert_texture_eq_png(img, &file_path);
      }
    }
  };
}

#[macro_export]
macro_rules! assert_texture_eq_png {
  (img: ident, img_name: ident) => {
    let mut expected = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let module_path = std::module_path!();
    let path = module_path.replace("::", "/");
    expected.push(&format!("test_cases/{}/{}", path, img_name));

    let overwrite = std::ffi::OsStr::new("overwrite");
    if std::env::var_os("RIBIR_IMG_TEST").map_or(false, |var| var == overwrite) {
      let folders = expected.parent().unwrap();
      std::fs::create_dir_all(folders).unwrap();
      let mut file = std::fs::File::create(expected.as_path()).unwrap();
      img.write_as_png(&mut file).unwrap();
    } else {
      let mut f = std::fs::File::open(expected.as_path()).unwrap();
      let mut bytes = Vec::new();
      std::io::Read::read_to_end(&mut f, &mut bytes).unwrap();
      let expected = PixelImage::from_png(&bytes);

      assert_eq!($img.pixel_bytes().len(), expected.pixel_bytes().len());
      assert_eq!($img.color_format(), ColorFormat::Rgba8);
      assert_eq!(expected.color_format(), ColorFormat::Rgba8);

      let dssim = dssim_core::Dssim::new();
      let rgba_img = unsafe {
        let ptr = $img.pixel_bytes().as_ptr() as *const _;
        std::slice::from_raw_parts(ptr, $img.pixel_bytes().len() / 4)
      };
      let img = dssim
        .create_image_rgba(rgba_img, $img.width() as usize, $img.height() as usize)
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

      let (v, _) = dssim.compare(&expected, img);
      assert!(v < 0.0001, "`{v}` over image test tolerance 0.0001");
    }
  };
}

#[macro_export]
macro_rules! test_case_name {
  ($sub_folder: literal, $name: expr, $format: literal) => {{
    let mut path_buffer = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let module_path = std::module_path!();
    let path = module_path.replace("::", "/");
    path_buffer.push(&format!(
      "test_cases/{path}/{}/{}.{}",
      $sub_folder, $name, $format
    ));

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
    let img = dssim
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

    const TOLERANCE: f64 = 0.0000005;
    let (v, _) = dssim.compare(&expected, img);
    let v: f64 = v.into();
    assert!(
      v < TOLERANCE,
      "`{v:}` over image test tolerance {TOLERANCE}"
    );
  }
}

/// Render painter by wgpu backend, and return the image.
#[cfg(feature = "wgpu")]
pub fn wgpu_render_commands(painter: &mut ribir_painter::Painter) -> PixelImage {
  use futures::executor::block_on;
  use ribir_geom::{DeviceRect, DeviceSize};
  use ribir_gpu::{GPUBackend, GPUBackendImpl, Texture};
  use ribir_painter::{AntiAliasing, Color, PainterBackend};

  let mut gpu_impl = block_on(ribir_gpu::WgpuImpl::headless());
  let bounds = painter.paint_bounds().to_i32();
  let rect = DeviceRect::from_size(DeviceSize::new(bounds.max_x() + 2, bounds.max_y() + 2));
  let mut texture = gpu_impl.new_texture(rect.size, AntiAliasing::None, ColorFormat::Rgba8);
  let mut backend = GPUBackend::new(gpu_impl, AntiAliasing::None);
  let commands = painter.finish();

  backend.begin_frame();
  backend.draw_commands(rect, commands, Color::TRANSPARENT, &mut texture);
  let img = texture.copy_as_image(&rect, backend.get_impl_mut());
  backend.end_frame();
  block_on(img).unwrap()
}
