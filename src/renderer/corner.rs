use crate::{config::Config, picture::Size, surface::Surface};

/// The radius `surface`'s corners are drawn with: what its rules give it, limited to half its
/// shorter side, border included, so that its arcs never overlap.
pub(super) fn radius(surface: &Surface, config: &Config) -> u8 {
    limit(surface.corner_radius(config), surface.size)
}

fn limit(radius: u8, size: Size) -> u8 {
    let half = size.width.min(size.height) / 2;
    u8::try_from(half).map_or(radius, |half| radius.min(half))
}

/// A disk of `radius` in a square twice as wide, row by row: how much of each pixel lies
/// inside the circle, from 0 to 255. Each quadrant is one corner of a window, so both
/// painters draw every corner from these values. A pixel counts how many of 16 × 16 points
/// spread evenly across it lie inside, with integers alone.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the painters draw from it from task 3 of the Rounded Corners milestone"
    )
)]
pub(super) fn disk(radius: u8) -> Vec<u8> {
    const SAMPLES: i32 = 16;
    // Coordinates count halves of the space between points, so that every point, in the
    // middle of its share of a pixel, has whole ones.
    let unit = 2 * SAMPLES;
    let center = unit * i32::from(radius);
    let side = 2 * i32::from(radius);
    let inside = |x: i32, y: i32| {
        let (across, down) = (x - center, y - center);
        across * across + down * down <= center * center
    };
    let total = SAMPLES * SAMPLES;
    (0..side)
        .flat_map(|row| (0..side).map(move |column| (column, row)))
        .map(|(column, row)| {
            let count: i32 = (0..SAMPLES)
                .flat_map(|y| (0..SAMPLES).map(move |x| (x, y)))
                .map(|(x, y)| i32::from(inside(unit * column + 2 * x + 1, unit * row + 2 * y + 1)))
                .sum();
            u8::try_from((count * 255 + total / 2) / total).unwrap_or(u8::MAX)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disks_are_symmetric_and_fill_toward_their_middle() {
        // Given radii from one pixel up, each disk mirrors across both axes and its diagonal,
        // and within a corner never thins toward the middle.
        for radius in [1_u8, 2, 3, 4, 8, 13, 64] {
            let side = 2 * usize::from(radius);
            let values = disk(radius);
            assert_eq!(values.len(), side * side);
            let at = |x: usize, y: usize| values.get(y * side + x).copied().unwrap();
            let half = usize::from(radius);
            for y in 0..side {
                for x in 0..side {
                    let value = at(x, y);
                    assert_eq!(value, at(side - 1 - x, y), "radius {radius} at {x},{y}");
                    assert_eq!(value, at(x, side - 1 - y), "radius {radius} at {x},{y}");
                    assert_eq!(value, at(y, x), "radius {radius} at {x},{y}");
                    if x + 1 < half && y < half {
                        assert!(value <= at(x + 1, y), "radius {radius} at {x},{y}");
                    }
                    if y + 1 < half && x < half {
                        assert!(value <= at(x, y + 1), "radius {radius} at {x},{y}");
                    }
                }
            }
            // The pixels at the middle lie wholly inside from radius 2, and the corner pixels
            // wholly outside from radius 4, where the circle passes their inner corners.
            if radius >= 2 {
                assert_eq!(at(half - 1, half - 1), 255, "radius {radius}");
            }
            if radius >= 4 {
                assert_eq!(at(0, 0), 0, "radius {radius}");
            }
        }
        assert!(disk(0).is_empty());
        assert_eq!(disk(1), [202; 4]);
        assert_eq!(
            disk(8).get(..16),
            Some(&[0, 0, 0, 0, 48, 151, 218, 250, 250, 218, 151, 48, 0, 0, 0, 0][..])
        );
    }

    #[test]
    fn a_disk_covers_the_area_of_its_circle() {
        // Given the area a circle covers, the disk's values add up to it within a small
        // fraction of the pixels its edge crosses.
        for radius in [8_u8, 13, 64] {
            let covered: f64 = disk(radius)
                .iter()
                .map(|&value| f64::from(value) / 255.0)
                .sum();
            let r = f64::from(radius);
            let area = std::f64::consts::PI * r * r;
            assert!(
                (covered - area).abs() < r / 16.0,
                "radius {radius}: {covered} against {area}"
            );
        }
    }

    #[test]
    fn radii_stop_at_half_the_shorter_side() {
        let size = |width, height| Size { width, height };
        assert_eq!(limit(8, size(100, 50)), 8);
        assert_eq!(limit(64, size(100, 50)), 25);
        assert_eq!(limit(8, size(15, 300)), 7);
        assert_eq!(limit(8, size(1, 1)), 0);
        assert_eq!(limit(0, size(1000, 1000)), 0);
        assert_eq!(limit(64, size(u16::MAX, u16::MAX)), 64);
    }
}
