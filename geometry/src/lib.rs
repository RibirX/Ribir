/// The tag for device unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicUnit;

/// The tag for logic unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicUnit;

pub type Rect = euclid::Rect<f32, LogicUnit>;
pub type Point = euclid::Point2D<f32, LogicUnit>;
pub type Size = euclid::Size2D<f32, LogicUnit>;
pub type Transform = euclid::Transform2D<f32, LogicUnit, LogicUnit>;
pub type ScaleToPhysic = euclid::Scale<u32, LogicUnit, PhysicUnit>;
pub type ScaleToLogic = euclid::Scale<f32, PhysicUnit, LogicUnit>;

pub type Vector = euclid::Vector2D<f32, LogicUnit>;
pub type Angle = euclid::Angle<f32>;
pub type Box2D = euclid::Box2D<f32, LogicUnit>;

pub type DeviceRect = euclid::Rect<u32, PhysicUnit>;
pub type DevicePoint = euclid::Point2D<u32, PhysicUnit>;
pub type DeviceSize = euclid::Size2D<u32, PhysicUnit>;
pub type DeviceVector = euclid::Vector2D<u32, PhysicUnit>;

pub type DeviceOffset = euclid::Point2D<i32, PhysicUnit>;
pub type ScaleOffsetToPhysic = euclid::Scale<i32, LogicUnit, PhysicUnit>;
pub type ScaleOffsetToLogic = euclid::Scale<f32, PhysicUnit, LogicUnit>;

pub const INFINITY_SIZE: Size = Size::new(f32::INFINITY, f32::INFINITY);
pub const ZERO_SIZE: Size = Size::new(0., 0.);

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test() {
    let logic_rect = Rect::new(Point::new(1., 1.), Size::new(3., 4.));
    let physic_rect = ScaleToPhysic::new(1).transform_rect(&logic_rect.cast());
    assert_eq!(logic_rect.origin.x as u32, physic_rect.origin.x);
    assert_eq!(logic_rect.origin.y as u32, physic_rect.origin.y);
    assert_eq!(logic_rect.width() as u32, physic_rect.width());
    assert_eq!(logic_rect.height() as u32, physic_rect.height());
  }
}
