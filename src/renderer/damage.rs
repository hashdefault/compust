use crate::{
    region::{Rect, Region},
    surface::Surface,
};
use x11rb::protocol::{render::Picture as PictureId, xproto::Window};

/// A surface as a frame showed it. Comparing the last frame's surfaces with the next one's
/// finds moves, resizes, recaptures, shape and opacity changes, fades, and restacks.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Shown {
    window: Window,
    picture: PictureId,
    bounds: Rect,
    shape: Vec<Rect>,
    opacity: u16,
}

impl Shown {
    pub(super) fn new(surface: &Surface, opacity: u16) -> Self {
        Self {
            window: surface.window,
            picture: surface.picture.id,
            bounds: surface.bounds(),
            shape: surface
                .shape
                .iter()
                .map(|rect| Rect::at(*rect, (0, 0)))
                .collect(),
            opacity,
        }
    }
}

/// Add to `area` what differs between two frames, each listing its surfaces bottom to top.
pub(super) fn changes(before: &[Shown], after: &[Shown], area: &mut Region) {
    for old in before {
        if !after.iter().any(|new| new.window == old.window) {
            area.add(old.bounds);
        }
    }
    let mut kept = Vec::new();
    for new in after {
        match before.iter().position(|old| old.window == new.window) {
            Some(index) => {
                if let Some(old) = before.get(index)
                    && old != new
                {
                    area.add(old.bounds);
                    area.add(new.bounds);
                }
                kept.push((index, new.bounds));
            }
            None => area.add(new.bounds),
        }
    }
    // A restack changes only where two surfaces that swapped places overlap.
    for (lower, (was, bounds)) in kept.iter().enumerate() {
        for (was_above, above) in kept.iter().skip(lower + 1) {
            if was_above < was
                && let Some(overlap) = bounds.intersect(*above)
            {
                area.add(overlap);
            }
        }
    }
}

/// Grow `area` until each blur footprint is wholly inside or outside it. A blurred pixel
/// depends on the scene around it, so repainting any part of a blur needs the scene current
/// across its whole footprint, which may in turn reach another blur.
pub(super) fn spread(area: &mut Region, footprints: &[Rect]) {
    while let Some(footprint) = footprints
        .iter()
        .find(|footprint| area.intersects(**footprint) && !area.covers(**footprint))
    {
        area.add(*footprint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(window: Window, x: i32, opacity: u16) -> Shown {
        Shown {
            window,
            picture: window + 100,
            bounds: Rect::new(x, 0, 10, 10),
            shape: vec![Rect::new(0, 0, 10, 10)],
            opacity,
        }
    }

    fn damage(before: &[Shown], after: &[Shown]) -> Vec<Rect> {
        let mut area = Region::default();
        changes(before, after, &mut area);
        area.rects().to_vec()
    }

    fn rects(list: &[(i32, i32, u16, u16)]) -> Vec<Rect> {
        list.iter()
            .map(|&(x, y, width, height)| Rect::new(x, y, width, height))
            .collect()
    }

    #[test]
    fn unchanged_surfaces_add_nothing() {
        let frame = [shown(1, 0, u16::MAX), shown(2, 5, u16::MAX)];
        let again = [shown(1, 0, u16::MAX), shown(2, 5, u16::MAX)];
        assert_eq!(damage(&frame, &again), rects(&[]));
    }

    #[test]
    fn moves_fades_and_departures_add_their_bounds() {
        // Given a moved surface, one fading, one gone, and one new, each adds where it was
        // and where it is.
        let before = [
            shown(1, 0, u16::MAX),
            shown(2, 100, u16::MAX),
            shown(3, 200, u16::MAX),
        ];
        let after = [
            shown(1, 30, u16::MAX),
            shown(2, 100, 1000),
            shown(4, 300, u16::MAX),
        ];
        assert_eq!(
            damage(&before, &after),
            rects(&[
                (200, 0, 10, 10),
                (0, 0, 10, 10),
                (30, 0, 10, 10),
                (100, 0, 10, 10),
                (300, 0, 10, 10),
            ])
        );
        let mut recaptured = shown(1, 0, u16::MAX);
        recaptured.picture = 7;
        assert_eq!(
            damage(&[shown(1, 0, u16::MAX)], &[recaptured]),
            rects(&[(0, 0, 10, 10)])
        );
    }

    #[test]
    fn restacks_add_only_where_swapped_surfaces_overlap() {
        // Given three surfaces where the top one drops to the bottom, only its overlap with
        // the surface it now lies under changes; the distant one is untouched.
        let before = [
            shown(1, 0, u16::MAX),
            shown(2, 100, u16::MAX),
            shown(3, 6, u16::MAX),
        ];
        let after = [
            shown(3, 6, u16::MAX),
            shown(1, 0, u16::MAX),
            shown(2, 100, u16::MAX),
        ];
        assert_eq!(damage(&before, &after), rects(&[(6, 0, 4, 10)]));
    }

    #[test]
    fn blur_footprints_join_wholly_and_in_chains() {
        // Given a change touching one footprint that overlaps a second, both join; a third
        // footprint elsewhere stays out.
        let mut area = Region::default();
        area.add(Rect::new(0, 0, 4, 4));
        let footprints = [
            Rect::new(60, 0, 50, 50),
            Rect::new(2, 2, 60, 60),
            Rect::new(500, 500, 10, 10),
        ];
        spread(&mut area, &footprints);
        assert!(area.covers(Rect::new(2, 2, 60, 60)));
        assert!(area.covers(Rect::new(60, 0, 50, 50)));
        assert!(!area.intersects(Rect::new(500, 500, 10, 10)));
    }
}
