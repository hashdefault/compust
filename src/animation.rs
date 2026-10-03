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

    /// Retargets from the current opacity. Each transition takes the duration configured when
    /// it starts, so a reloaded `fade_ms` also applies to windows opened before the reload.
    pub(crate) fn close(&mut self, now: Instant, duration: Duration) {
        self.retarget(now, 0, duration);
    }

    pub(crate) fn reopen(&mut self, now: Instant, duration: Duration) {
        self.retarget(now, u16::MAX, duration);
    }

    fn retarget(&mut self, now: Instant, to: u16, duration: Duration) {
        self.from = self.sample(now);
        self.to = to;
        self.started = now;
        self.duration = duration;
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
        fade.close(halfway, Duration::from_millis(100));
        assert_eq!(fade.sample(halfway), before);
        assert_eq!(fade.sample(halfway + Duration::from_millis(100)), 0);
    }

    #[test]
    fn closing_takes_the_duration_given_when_it_starts() {
        // Given a fade opened with 100 ms, when closed with 400 ms, then the close is
        // continuous and lasts the new duration.
        let now = Instant::now();
        let mut fade = Fade::opening(now, Duration::from_millis(100));
        let opened = now + Duration::from_millis(100);
        fade.close(opened, Duration::from_millis(400));
        assert_eq!(fade.sample(opened), u16::MAX);
        let halfway = fade.sample(opened + Duration::from_millis(200));
        assert!(
            (u16::MAX / 4..u16::MAX / 4 * 3).contains(&halfway),
            "{halfway}"
        );
        assert!(fade.active(opened + Duration::from_millis(399)));
        assert_eq!(fade.sample(opened + Duration::from_millis(400)), 0);
    }

    #[test]
    fn zero_duration_reaches_target_without_division() {
        // Given disabled animations, when sampled, then the endpoint is immediate.
        let now = Instant::now();
        assert_eq!(Fade::opening(now, Duration::ZERO).sample(now), u16::MAX);
    }

    #[test]
    fn reopening_mid_close_is_continuous_and_finishes_exactly() {
        let now = Instant::now();
        let duration = Duration::from_millis(100);
        let mut fade = Fade::opening(now, duration);
        fade.close(now + duration, duration);
        let halfway = now + duration + Duration::from_millis(50);
        let before = fade.sample(halfway);

        fade.reopen(halfway, duration);

        assert_eq!(fade.sample(halfway), before);
        let values: Vec<_> = (0..=100)
            .map(|ms| fade.sample(halfway + Duration::from_millis(ms)))
            .collect();
        assert!(values.windows(2).all(|pair| pair.first() <= pair.last()));
        assert_eq!(values.last(), Some(&u16::MAX));
        assert!(!fade.active(halfway + duration));
    }
}
