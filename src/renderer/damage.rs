use super::shadow::Shadow;
use crate::{
    region::{Rect, Region},
    surface::Surface,
};
use x11rb::protocol::{render::Picture as PictureId, xproto::Window};

/// Above every surface: the output needs repainting there, but no surface's scene changed.
pub(super) const OUTPUT: usize = usize::MAX;

/// What a pending change belongs to; its layer is known only when the frame is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// The background beneath every surface.
    Background,
    /// The content of one window's surface.
    Window(Window),
    /// The output alone, as after an exposure.
    Output,
}

/// Part of the screen where the composite of `layer` and every layer above it changed, so a
/// surface higher than `layer` sees a different scene beneath it there. Layer 0 is the
/// background, and a frame's surface `k` is layer `k + 1`. A change that is not `shown`
/// alters only what a blur reads, not the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Change {
    pub(super) area: Rect,
    pub(super) layer: usize,
    pub(super) shown: bool,
}

/// A surface as a frame showed it. Comparing the last frame's surfaces with the next one's
/// finds moves, resizes, recaptures, shape and opacity changes, fades, blur or a shadow turned
/// on or off, corners rounded otherwise, and restacks.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Shown {
    window: Window,
    picture: PictureId,
    bounds: Rect,
    shape: Vec<Rect>,
    opacity: u16,
    blur: bool,
    shadow: Option<Shadow>,
    /// The radius of its rounded corners; zero when they are square.
    corners: u8,
}

impl Shown {
    pub(super) fn new(
        surface: &Surface,
        opacity: u16,
        blur: bool,
        shadow: Option<Shadow>,
        corners: u8,
    ) -> Self {
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
            blur,
            shadow,
            corners,
        }
    }

    /// The screen area the surface changes when it does: its bounds and its shadow.
    fn extent(&self) -> Rect {
        self.shadow
            .map_or(self.bounds, |shadow| self.bounds.hull(shadow.rect))
    }
}

/// What differs between two frames, each listing its surfaces bottom to top. A surface that
/// appeared, changed, left, or changed places changes the scene of every surface above it in
/// either frame, across its old and new extent, which takes in its shadow. The output changes
/// where a surface appeared, changed, or left, and where two surfaces that swapped places
/// overlap.
pub(super) fn changes(before: &[Shown], after: &[Shown]) -> Vec<Change> {
    let mut changes = Vec::new();
    // Where each current surface was in the previous frame.
    let was: Vec<_> = after
        .iter()
        .map(|new| before.iter().position(|old| old.window == new.window))
        .collect();
    // The highest layer beneath every current surface that was above previous surface `index`.
    let under_former = |index: usize| {
        was.iter()
            .position(|was| was.is_some_and(|was| was > index))
            .unwrap_or(OUTPUT)
    };
    for (index, old) in before.iter().enumerate() {
        if !after.iter().any(|new| new.window == old.window) {
            changes.push(Change {
                area: old.extent(),
                layer: under_former(index),
                shown: true,
            });
        }
    }
    let restacked = !was.iter().flatten().is_sorted();
    // Whether the surface now at `current`, previously at `index`, changed order with another.
    let reordered = |current: usize, index: usize| {
        restacked
            && was
                .iter()
                .enumerate()
                .any(|(other, was)| was.is_some_and(|was| (was < index) != (other < current)))
    };
    for ((current, new), was) in after.iter().enumerate().zip(&was) {
        let layer = current + 1;
        match was.and_then(|index| Some((index, before.get(index)?))) {
            Some((index, old)) => {
                let shown = old != new;
                if shown || reordered(current, index) {
                    let layer = layer.min(under_former(index));
                    for area in [old.extent(), new.extent()] {
                        changes.push(Change { area, layer, shown });
                    }
                }
            }
            None => changes.push(Change {
                area: new.extent(),
                layer,
                shown: true,
            }),
        }
    }
    if restacked {
        let kept: Vec<_> = after
            .iter()
            .zip(&was)
            .filter_map(|(new, was)| Some(((*was)?, new.extent())))
            .collect();
        for (lower, (was, bounds)) in kept.iter().enumerate() {
            for (was_above, above) in kept.iter().skip(lower + 1) {
                if was_above < was
                    && let Some(overlap) = bounds.intersect(*above)
                {
                    changes.push(Change {
                        area: overlap,
                        layer: OUTPUT,
                        shown: true,
                    });
                }
            }
        }
    }
    changes
}

/// A surface whose blurred backdrop a frame shows.
#[derive(Debug)]
pub(super) struct Blurred {
    pub(super) layer: usize,
    /// Its bounds on the screen, where its backdrop is kept.
    pub(super) bounds: Rect,
    pub(super) footprint: Rect,
    /// Whether a backdrop kept from an earlier frame covers `bounds`.
    pub(super) kept: bool,
}

/// Which surfaces of `blurred`, listed bottom to top, must blur their backdrop again. A
/// blurred pixel depends on the scene around it, so a change beneath a surface anywhere in
/// its footprint changes its whole backdrop, and with it the scene of every surface above.
/// A surface repainted without a kept backdrop also blurs again. Either way the scene must be
/// current across its whole footprint, so `area` grows to cover it; other surfaces repainted
/// only because of that reuse their kept backdrops.
pub(super) fn plan(blurred: &[Blurred], changes: &mut Vec<Change>, area: &mut Region) -> Vec<bool> {
    let mut fresh = Vec::with_capacity(blurred.len());
    for surface in blurred {
        let beneath = changes.iter().any(|change| {
            change.layer < surface.layer && change.area.intersect(surface.footprint).is_some()
        });
        if beneath {
            changes.push(Change {
                area: surface.bounds,
                layer: surface.layer,
                shown: true,
            });
            area.add(surface.footprint);
        }
        fresh.push(beneath);
    }
    while let Some(index) = blurred
        .iter()
        .zip(&fresh)
        .position(|(surface, fresh)| !fresh && !surface.kept && area.intersects(surface.bounds))
    {
        if let (Some(fresh), Some(surface)) = (fresh.get_mut(index), blurred.get(index)) {
            *fresh = true;
            area.add(surface.footprint);
        }
    }
    fresh
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
            blur: false,
            shadow: None,
            corners: 0,
        }
    }

    fn change(x: i32, width: u16, layer: usize, shown: bool) -> Change {
        Change {
            area: Rect::new(x, 0, width, 10),
            layer,
            shown,
        }
    }

    #[test]
    fn unchanged_surfaces_change_nothing() {
        let frame = [shown(1, 0, u16::MAX), shown(2, 5, u16::MAX)];
        let again = [shown(1, 0, u16::MAX), shown(2, 5, u16::MAX)];
        assert_eq!(changes(&frame, &again), []);
    }

    #[test]
    fn moves_fades_and_departures_change_their_bounds_from_their_layer() {
        // Given a moved surface, one fading, one gone from the top, and one new, each changes
        // where it was and where it is, from its own layer up; nothing was above the one gone.
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
            changes(&before, &after),
            [
                change(200, 10, OUTPUT, true),
                change(0, 10, 1, true),
                change(30, 10, 1, true),
                change(100, 10, 2, true),
                change(100, 10, 2, true),
                change(300, 10, 3, true),
            ]
        );
        let mut recaptured = shown(1, 0, u16::MAX);
        recaptured.picture = 7;
        assert_eq!(
            changes(
                &[shown(1, 0, u16::MAX), shown(2, 50, u16::MAX)],
                &[recaptured]
            ),
            [
                change(50, 10, OUTPUT, true),
                change(0, 10, 1, true),
                change(0, 10, 1, true),
            ]
        );
        // A surface gone from beneath another changes the scene of the one above.
        assert_eq!(
            changes(
                &[shown(1, 0, u16::MAX), shown(2, 50, u16::MAX)],
                &[shown(2, 50, u16::MAX)]
            ),
            [change(0, 10, 0, true)]
        );
        // A rule turning blur on shows a different backdrop through the same surface.
        let mut blurred = shown(1, 0, 1000);
        blurred.blur = true;
        assert_eq!(
            changes(&[shown(1, 0, 1000)], &[blurred]),
            [change(0, 10, 1, true), change(0, 10, 1, true)]
        );
        // Rounding its corners changes what shows there, from the surface's own layer up.
        let mut rounded = shown(1, 0, u16::MAX);
        rounded.corners = 4;
        assert_eq!(
            changes(&[shown(1, 0, u16::MAX)], &[rounded]),
            [change(0, 10, 1, true), change(0, 10, 1, true)]
        );
    }

    #[test]
    fn restacks_show_only_overlaps_but_change_the_scenes_beneath() {
        // Given three surfaces where the top one drops to the bottom, the output changes only
        // where it overlaps the surface now above it. The scene beneath the dropped surface
        // lost both others, and the scene beneath them gained it.
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
        assert_eq!(
            changes(&before, &after),
            [
                change(6, 10, 1, false),
                change(6, 10, 1, false),
                change(0, 10, 0, false),
                change(0, 10, 0, false),
                change(100, 10, 0, false),
                change(100, 10, 0, false),
                change(6, 4, OUTPUT, true),
            ]
        );
    }

    fn blurred(layer: usize, x: i32, kept: bool) -> Blurred {
        Blurred {
            layer,
            bounds: Rect::new(x, 0, 10, 10),
            footprint: Rect::new(x - 8, 0, 26, 18),
            kept,
        }
    }

    fn area(changes: &[Change]) -> Region {
        let mut area = Region::default();
        for change in changes.iter().filter(|change| change.shown) {
            area.add(change.area);
        }
        area
    }

    #[test]
    fn kept_backdrops_serve_changes_at_or_above_their_layer() {
        // Given a blurred surface whose own content and a surface above it changed, its kept
        // backdrop serves and the area stays as small as the changes.
        let surfaces = [blurred(2, 0, true)];
        let mut changes = vec![
            Change {
                area: Rect::new(2, 2, 2, 2),
                layer: 2,
                shown: true,
            },
            change(5, 20, 3, true),
            change(0, 10, OUTPUT, true),
        ];
        let mut painted = area(&changes);
        assert_eq!(plan(&surfaces, &mut changes, &mut painted), [false]);
        assert!(!painted.intersects(Rect::new(-8, 10, 8, 8)));
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn changes_beneath_a_footprint_blur_again_and_reach_the_surfaces_above() {
        // Given a change beneath the lower of two blurred surfaces, in its margin, it blurs
        // again; that changes its bounds for the upper one, whose footprint reaches them. A
        // third surface elsewhere keeps its backdrop.
        let surfaces = [
            blurred(1, 0, true),
            blurred(3, 14, true),
            blurred(5, 500, true),
        ];
        let mut changes = vec![Change {
            area: Rect::new(-6, 2, 2, 2),
            layer: 0,
            shown: true,
        }];
        let mut painted = area(&changes);
        assert_eq!(
            plan(&surfaces, &mut changes, &mut painted),
            [true, true, false]
        );
        assert!(painted.covers(Rect::new(-8, 0, 26, 18)));
        assert!(painted.covers(Rect::new(6, 0, 26, 18)));
        assert_eq!(changes.get(1), Some(&change(0, 10, 1, true)));
        assert!(!painted.intersects(Rect::new(500, 0, 10, 10)));
    }

    #[test]
    fn surfaces_without_kept_backdrops_blur_when_repainted() {
        // Given surfaces without kept backdrops, the one the area touches blurs, and its
        // footprint reaches the next; one elsewhere is left alone.
        let surfaces = [
            blurred(1, 0, false),
            blurred(2, 15, false),
            blurred(3, 500, false),
        ];
        let mut changes = vec![change(2, 2, OUTPUT, true)];
        let mut painted = area(&changes);
        assert_eq!(
            plan(&surfaces, &mut changes, &mut painted),
            [true, true, false]
        );
        assert_eq!(changes.len(), 1);
    }
}
