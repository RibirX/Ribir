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
pub type ScaleToPhysic = euclid::Scale<f32, LogicUnit, PhysicUnit>;
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
  fn scale_from_physics_rect_to_logic_rect_and_back() {
    let physic_rect = DeviceRect::new(DevicePoint::new(1, 1), DeviceSize::new(3, 4));
    let factor = 1.3;
    let logic_rect: Rect = ScaleToLogic::new(factor)
      .transform_rect(&physic_rect.cast())
      .cast();

    assert!((logic_rect.origin.x - 1.3).abs() < f32::EPSILON);
    assert!((logic_rect.origin.y - 1.3).abs() < f32::EPSILON);
    assert!((logic_rect.width() - 3.8999999).abs() < f32::EPSILON);
    assert!((logic_rect.height() - 5.2).abs() < f32::EPSILON);

    let physic_rect2: DeviceRect = ScaleToPhysic::new(1.0 / factor)
      .transform_rect(&logic_rect.cast())
      .cast();
    assert_eq!(physic_rect, physic_rect2)
  }
}
