#![feature(decl_macro)]

#[allow(unused_imports)]
use png;
use std::io::Write;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};

pub macro write_canvas_to($frame: expr, $path: expr) {
  let abs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .with_file_name(file!())
    .with_file_name($path);
  let writer = std::fs::File::create(&abs_path).unwrap();
  backend_write_png($frame, writer);
}

/// check if the frame is equal to the image at `path`, the path relative the
/// package root;
pub macro assert_canvas_eq($frame: expr, $path: expr $(,)?) {
  let file_data = file_bytes!($path);

  let mut frame_data = vec![];
  let cursor = std::io::Cursor::new(&mut frame_data);

  backend_write_png($frame, cursor);

  if file_data != frame_data {
    panic!(
      "{}",
      format!(
        "Canvas is not same with `{}`.\n
You can use `write_canvas_to` to save Canvas as png to compare.",
        $path
      )
    );
  }
}

pub fn backend_write_png<W: std::io::Write>(backend: &dyn painter::PainterBackend, writer: W) {
  backend
    .capture(Box::new(move |size, rows| {
      let mut png_encoder = png::Encoder::new(writer, size.width, size.height);
      png_encoder.set_depth(png::BitDepth::Eight);
      png_encoder.set_color(png::ColorType::RGBA);

      let mut writer = png_encoder.write_header().unwrap();
      let mut stream_writer = writer.stream_writer_with_size(size.width as usize * 4);

      rows.for_each(|data| {
        stream_writer.write(data).unwrap();
      });
      stream_writer.finish().unwrap();
    }))
    .unwrap();
}

/// Check if two image has same data.
pub macro assert_img_eq($img1: expr, $img2: expr) {
  if file_bytes!($img1) != file_bytes!($img2) {
    panic!("`{}` and `{}` is not same.", $img1, $img2);
  }
}

#[allow(unused_macros)]
macro file_bytes($path: expr) {{
  let abs_path = abs_path!($path);
  std::fs::read(abs_path).expect(&format!("{}", abs_path!($path).to_str().unwrap()))
}}

#[allow(unused_macros)]
macro abs_path($path: expr) {{
  std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .with_file_name(file!())
    .with_file_name($path)
    .canonicalize()
    .unwrap()
}}
