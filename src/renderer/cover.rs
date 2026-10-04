use crate::region::Rect;

/// Rectangles past this count are left out, so a cover can hide less than it might, never more.
const LIMIT: usize = 64;

/// Screen area hidden behind opaque surfaces, as rectangles that may overlap.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Cover {
    rects: Vec<Rect>,
}

impl Cover {
    fn add(&mut self, rect: Rect) {
        if self.rects.len() < LIMIT {
            self.rects.push(rect);
        }
    }

    /// Stop hiding `rect`.
    fn expose(&mut self, rect: Rect) {
        let mut rects: Vec<_> = self
            .rects
            .iter()
            .flat_map(|kept| kept.minus(rect))
            .collect();
        rects.truncate(LIMIT);
        self.rects = rects;
    }

    /// The parts of `rects` this cover leaves in sight; past the limit, all of them.
    pub(super) fn visible(&self, rects: Vec<Rect>) -> Vec<Rect> {
        let mut visible = rects.clone();
        for cover in &self.rects {
            visible = visible
                .into_iter()
                .flat_map(|rect| rect.minus(*cover))
                .collect();
            if visible.len() > LIMIT {
                return rects;
            }
        }
        visible
    }
}

/// A surface as the cover pass sees it.
#[derive(Debug, Default)]
pub(super) struct Layer {
    /// Its opaque parts on the screen; none unless it is opaque.
    pub(super) opaque: Vec<Rect>,
    /// Its bounds and footprint on the screen when it is due to blur again.
    pub(super) blur: Option<(Rect, Rect)>,
}

#[derive(Debug)]
pub(super) struct Covers {
    /// The area hidden above each surface, bottom to top.
    pub(super) above: Vec<Cover>,
    /// The area hidden above the background.
    pub(super) background: Cover,
    /// Whether each surface due to blur again is hidden whole, so that it need not.
    pub(super) hidden: Vec<bool>,
}

/// What opaque surfaces hide of those beneath them, for surfaces listed bottom to top. A
/// surface due to blur again reads the scene beneath it across its footprint, so nothing
/// there is hidden from the surfaces beneath it, unless that surface is hidden whole itself
/// and so need not blur.
pub(super) fn covers(layers: &[Layer]) -> Covers {
    let mut cover = Cover::default();
    let mut above = Vec::with_capacity(layers.len());
    let mut hidden = Vec::with_capacity(layers.len());
    for layer in layers.iter().rev() {
        let whole = layer
            .blur
            .is_some_and(|(bounds, _)| cover.visible(vec![bounds]).is_empty());
        above.push(cover.clone());
        hidden.push(whole);
        if let Some((_, footprint)) = layer.blur
            && !whole
        {
            cover.expose(footprint);
        }
        for rect in &layer.opaque {
            cover.add(*rect);
        }
    }
    above.reverse();
    hidden.reverse();
    Covers {
        above,
        background: cover,
        hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: i32, y: i32, size: u16) -> Rect {
        Rect::new(x, y, size, size)
    }

    fn opaque(rect: Rect) -> Layer {
        Layer {
            opaque: vec![rect],
            blur: None,
        }
    }

    #[test]
    fn opaque_surfaces_hide_what_lies_beneath_them() {
        // Given a translucent surface under an opaque one, the part beneath the opaque one is
        // hidden from it and from the background; nothing hides the opaque one.
        let covers = covers(&[Layer::default(), opaque(square(10, 0, 20))]);
        assert_eq!(
            covers
                .above
                .first()
                .map(|cover| cover.visible(vec![square(0, 0, 20)])),
            Some(vec![Rect::new(0, 0, 10, 20)])
        );
        assert_eq!(covers.background.visible(vec![square(10, 0, 20)]), []);
        assert_eq!(covers.above.get(1), Some(&Cover::default()));
        assert_eq!(covers.hidden, [false, false]);
    }

    #[test]
    fn a_blur_due_again_sees_everything_beneath_its_footprint() {
        // Given a surface under one due to blur again, which lies partly under an opaque
        // surface, nothing of the footprint is hidden beneath the blurred one; the rest of
        // the opaque surface still hides.
        let layers = [
            Layer::default(),
            Layer {
                opaque: Vec::new(),
                blur: Some((square(20, 0, 20), square(12, 0, 36))),
            },
            opaque(square(30, 0, 40)),
        ];
        let covers = covers(&layers);
        assert_eq!(covers.hidden, [false, false, false]);
        let beneath = covers.above.first().cloned().unwrap_or_default();
        let shown = |rect| beneath.visible(vec![rect]) == [rect];
        let hidden = |rect| beneath.visible(vec![rect]).is_empty();
        assert!(shown(Rect::new(30, 0, 18, 36)));
        assert!(hidden(Rect::new(48, 0, 22, 36)));
        assert!(hidden(Rect::new(30, 36, 40, 4)));
    }

    #[test]
    fn a_blur_hidden_whole_is_skipped_and_reads_nothing() {
        // Given a surface due to blur again entirely under an opaque one, it need not blur,
        // and the opaque surface keeps hiding what lies beneath.
        let layers = [
            Layer::default(),
            Layer {
                opaque: Vec::new(),
                blur: Some((square(20, 0, 20), square(12, 0, 36))),
            },
            opaque(square(0, 0, 100)),
        ];
        let covers = covers(&layers);
        assert_eq!(covers.hidden, [false, true, false]);
        assert_eq!(
            covers
                .above
                .first()
                .map(|cover| cover.visible(vec![square(0, 0, 100)])),
            Some(Vec::new())
        );
    }

    #[test]
    fn covers_past_the_limit_hide_less() {
        // Given opaque pieces along a diagonal, each cut adds two parts; a rectangle cut into
        // more parts than the limit is shown whole rather than partly hidden.
        let pieces: Vec<_> = (0..64).map(|index| square(index, index, 1)).collect();
        let covers = covers(&[
            Layer::default(),
            Layer {
                opaque: pieces,
                blur: None,
            },
        ]);
        let whole = square(0, 0, 64);
        assert_eq!(
            covers.above.first().map(|cover| cover.visible(vec![whole])),
            Some(vec![whole])
        );
    }
}
