use anyhow::Result;
use x11rb::protocol::xproto::Rectangle;

/// Rectangles past this count merge, the pair whose hull adds the least area first.
const LIMIT: usize = 16;

/// A rectangle in root coordinates; the right and bottom edges are exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl Rect {
    pub(crate) fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            left: x,
            top: y,
            right: x + i32::from(width),
            bottom: y + i32::from(height),
        }
    }

    /// An X11 rectangle given relative to `origin`.
    pub(crate) fn at(rect: Rectangle, origin: (i16, i16)) -> Self {
        Self::new(
            i32::from(rect.x) + i32::from(origin.0),
            i32::from(rect.y) + i32::from(origin.1),
            rect.width,
            rect.height,
        )
    }

    fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    pub(crate) fn intersect(self, other: Self) -> Option<Self> {
        let rect = Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        (!rect.is_empty()).then_some(rect)
    }

    pub(crate) fn contains(self, other: Self) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }

    /// The parts of this rectangle outside `other`: at most four bands, full-width above
    /// and below `other` and beside it in between.
    pub(crate) fn minus(self, other: Self) -> Vec<Self> {
        let Some(cut) = self.intersect(other) else {
            return vec![self];
        };
        let bands = [
            (self.left, self.top, self.right, cut.top),
            (self.left, cut.bottom, self.right, self.bottom),
            (self.left, cut.top, cut.left, cut.bottom),
            (cut.right, cut.top, self.right, cut.bottom),
        ];
        bands
            .into_iter()
            .map(|(left, top, right, bottom)| Self {
                left,
                top,
                right,
                bottom,
            })
            .filter(|band| !band.is_empty())
            .collect()
    }

    /// The smallest rectangle holding both.
    pub(crate) fn hull(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    fn area(self) -> i64 {
        i64::from(self.right - self.left) * i64::from(self.bottom - self.top)
    }

    pub(crate) fn x11(self) -> Result<Rectangle> {
        Ok(Rectangle {
            x: i16::try_from(self.left)?,
            y: i16::try_from(self.top)?,
            width: u16::try_from(self.right - self.left)?,
            height: u16::try_from(self.bottom - self.top)?,
        })
    }
}

/// An area of the root as rectangles that may overlap, which the server unions. It can
/// cover more than was added, never less.
#[derive(Clone, Debug, Default)]
pub(crate) struct Region {
    rects: Vec<Rect>,
}

impl Region {
    pub(crate) fn add(&mut self, rect: Rect) {
        if rect.is_empty() || self.covers(rect) {
            return;
        }
        self.rects.retain(|kept| !rect.contains(*kept));
        self.rects.push(rect);
        if self.rects.len() > LIMIT {
            self.merge_closest();
        }
    }

    fn merge_closest(&mut self) {
        let mut closest = None;
        for (first, a) in self.rects.iter().enumerate() {
            for (second, b) in self.rects.iter().enumerate().skip(first + 1) {
                let growth = a.hull(*b).area() - a.area() - b.area();
                if closest.is_none_or(|(least, _, _)| growth < least) {
                    closest = Some((growth, first, second));
                }
            }
        }
        if let Some((_, first, second)) = closest {
            // `second` is the larger index, so removing it first leaves `first` in place.
            let b = self.rects.swap_remove(second);
            let a = self.rects.swap_remove(first);
            self.add(a.hull(b));
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    pub(crate) fn intersects(&self, rect: Rect) -> bool {
        self.rects.iter().any(|kept| kept.intersect(rect).is_some())
    }

    /// Whether one of the rectangles contains `rect`; once true, later additions keep it true.
    pub(crate) fn covers(&self, rect: Rect) -> bool {
        self.rects.iter().any(|kept| kept.contains(rect))
    }

    /// The parts of `rects` inside this region, for a clip list.
    pub(crate) fn clip(&self, rects: impl Iterator<Item = Rect>) -> Vec<Rect> {
        rects
            .flat_map(|rect| {
                self.rects
                    .iter()
                    .filter_map(move |kept| kept.intersect(rect))
            })
            .collect()
    }

    pub(crate) fn x11(&self) -> Result<Vec<Rectangle>> {
        self.rects.iter().map(|rect| rect.x11()).collect()
    }

    pub(crate) fn rects(&self) -> &[Rect] {
        &self.rects
    }
}

/// `rects` without any part of `cut`.
pub(crate) fn without(rects: Vec<Rect>, cut: &[Rect]) -> Vec<Rect> {
    cut.iter().fold(rects, |rects, cut| {
        rects
            .into_iter()
            .flat_map(|rect| rect.minus(*cut))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: i32, y: i32, size: u16) -> Rect {
        Rect::new(x, y, size, size)
    }

    #[test]
    fn contained_rectangles_are_dropped() {
        // Given a rectangle inside another, either order keeps only the outer one.
        let mut region = Region::default();
        region.add(square(10, 10, 5));
        region.add(square(0, 0, 50));
        region.add(square(20, 20, 5));
        region.add(Rect::new(30, 30, 0, 9));
        assert_eq!(region.rects, [square(0, 0, 50)]);
    }

    #[test]
    fn rectangles_past_the_limit_merge_with_their_nearest() {
        // Given a row of separate squares and one far away, the merge joins two neighbors
        // and leaves the distant square alone.
        let mut region = Region::default();
        for index in 0..16 {
            region.add(square(index * 20, 0, 10));
        }
        region.add(square(1000, 1000, 10));
        assert_eq!(region.rects.len(), LIMIT);
        assert!(region.covers(square(1000, 1000, 10)));
        assert!(region.rects.contains(&Rect::new(0, 0, 30, 10)));
        for index in 0..16 {
            assert!(region.covers(square(index * 20, 0, 10)));
        }
    }

    #[test]
    fn clips_keep_only_shared_parts() {
        let mut inside = Region::default();
        inside.add(square(0, 0, 20));
        inside.add(square(100, 100, 20));
        let clip = inside.clip([square(10, 10, 100)].into_iter());
        assert_eq!(clip, [square(10, 10, 10), square(100, 100, 10)]);
        assert!(!inside.intersects(square(20, 0, 80)));
        assert!(inside.intersects(square(19, 19, 2)));
    }

    #[test]
    fn subtraction_leaves_the_bands_around_a_cut() {
        // Given a cut in the middle, four bands remain; at an edge or outside, fewer.
        let whole = square(0, 0, 30);
        assert_eq!(
            whole.minus(square(10, 10, 10)),
            [
                Rect::new(0, 0, 30, 10),
                Rect::new(0, 20, 30, 10),
                Rect::new(0, 10, 10, 10),
                Rect::new(20, 10, 10, 10),
            ]
        );
        assert_eq!(
            whole.minus(Rect::new(-5, -5, 40, 15)),
            [Rect::new(0, 10, 30, 20)]
        );
        assert_eq!(whole.minus(square(-5, -5, 50)), []);
        assert_eq!(whole.minus(square(40, 40, 5)), [whole]);
    }
}
