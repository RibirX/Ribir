// use ribir_app::prelude::Application;
// use ribir_core::{prelude::AppContext, widget::Widget, window::Window as RibirWindow};
// use ribir_geometry::{DeviceSize, Point, Size};

// pub trait WindowBuilder {
//   fn build(self, app: dyn Application) -> RibirWindow;

//   /// Requests the window to be of specific dimensions.
//   fn with_inner_size(self, size: Size) -> Self;

//   /// Sets a minimum dimension size for the window.
//   fn with_min_inner_size( self, min_size: Size) -> Self;

//   /// Sets a maximum dimension size for the window.
//   fn with_max_inner_size( self, max_size: Size) -> Self;

//   /// Sets a desired initial position for the window.
//   fn with_position(self, position: Point) -> Self;

//   /// Sets whether the window is resizable or not.
//   fn with_resizable( self, resizable: bool) -> Self;

//   /// Requests a specific title for the window.
//   fn with_title<T: Into<String>>( self, title: T) -> Self;

//   /// Requests maximized mode.
//   fn with_maximized( self, maximized: bool) -> Self;

//   /// Sets whether the window will be initially hidden or visible.
//   fn with_visible( self, visible: bool) -> Self;

//   /// Sets whether the background of the window should be transparent.
//   fn with_transparent( self, transparent: bool) -> Self;

//   /// Sets whether the window should have a border, a title bar, etc.

//   fn with_decorations( self, decorations: bool) -> Self;

//   /// Sets the window icon.
//   // fn with_window_icon( self, window_icon: WindowIcon>) -> Self;
// }
