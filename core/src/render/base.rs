use std::fmt::Debug;

bitflags! {
    pub struct LayoutConstraints: u8 {
        const DECIDED_BY_SELF = 0;
        const EFFECTED_BY_PARENT = 1;
        const EFFECTED_BY_CHILDREN = 2;
    }
}
