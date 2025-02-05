/// An amount of space in 2 dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Size<T = f32> {
    /// The width.
    pub width: T,
    /// The height.
    pub height: T,
}

impl<T> Size<T> {
    /// A [`Size`] with zero width and height.
    pub const ZERO: Size = Size::new(0., 0.);

    /// A [`Size`] with a width and height of 1 unit.
    pub const UNIT: Size = Size::new(1., 1.);

    /// A [`Size`] with infinite width and height.
    pub const INFINITY: Size = Size::new(f32::INFINITY, f32::INFINITY);

    /// Creates a new  [`Size`] with the given width and height.
    pub const fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}
