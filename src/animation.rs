use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct Fade {
    from: u16,
    to: u16,
    started: Instant,
    duration: Duration,
}

impl Fade {
    pub(crate) fn opening(now: Instant, duration: Duration) -> Self {
        Self {
            from: 0,
            to: u16::MAX,
            started: now,
            duration,
        }
    }

    pub(crate) fn close(&mut self, now: Instant) {
        self.from = self.sample(now);
        self.to = 0;
        self.started = now;
    }

    pub(crate) fn active(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started) < self.duration
    }

    pub(crate) fn sample(&self, now: Instant) -> u16 {
        if !self.active(now) {
            return self.to;
        }
        // Fixed point smoothstep: t*t*(3-2*t), with bounded u128 intermediates.
        let scale = u128::from(u16::MAX);
        let t = now.saturating_duration_since(self.started).as_nanos() * scale
            / self.duration.as_nanos();
        let eased = t * t * (3 * scale - 2 * t) / (scale * scale);
        let mixed = u128::from(self.from) * (scale - eased) + u128::from(self.to) * eased;
        u16::try_from(mixed / scale).unwrap_or(u16::MAX)
    }
}

pub(crate) fn multiply_alpha(left: u16, right: u16) -> u16 {
    u16::try_from(u32::from(left) * u32::from(right) / u32::from(u16::MAX)).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_is_monotonic_and_finishes_exactly() {
        // Given a 100 ms fade, when sampled through its lifetime, then it is bounded and monotonic.
        let now = Instant::now();
        let fade = Fade::opening(now, Duration::from_millis(100));
        let values: Vec<_> = (0..=110)
            .map(|ms| fade.sample(now + Duration::from_millis(ms)))
            .collect();
        assert!(values.windows(2).all(|pair| pair.first() <= pair.last()));
        assert_eq!(values.first(), Some(&0));
        assert_eq!(values.last(), Some(&u16::MAX));
    }

    #[test]
    fn closing_mid_animation_is_continuous() {
        // Given a partly opened window, when closed, then opacity has no discontinuity.
        let now = Instant::now();
        let mut fade = Fade::opening(now, Duration::from_millis(100));
        let halfway = now + Duration::from_millis(50);
        let before = fade.sample(halfway);
        fade.close(halfway);
        assert_eq!(fade.sample(halfway), before);
        assert_eq!(fade.sample(halfway + Duration::from_millis(100)), 0);
    }

    #[test]
    fn zero_duration_reaches_target_without_division() {
        // Given disabled animations, when sampled, then the endpoint is immediate.
        let now = Instant::now();
        assert_eq!(Fade::opening(now, Duration::ZERO).sample(now), u16::MAX);
    }
}
