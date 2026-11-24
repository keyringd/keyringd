use super::ScreenPoint;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ScreenRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl ScreenRect {
    pub(crate) fn center(self) -> ScreenPoint {
        ScreenPoint {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }
}
