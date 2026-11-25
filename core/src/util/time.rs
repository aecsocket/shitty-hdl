use std::num::NonZero;

#[derive(Debug, Clone, Copy)]
pub struct Period {
    femtos: NonZero<u64>,
}

impl Period {
    #[must_use]
    pub const fn from_femtos(femtos: NonZero<u64>) -> Self {
        Self { femtos }
    }

    #[must_use]
    pub const fn femtos(self) -> u64 {
        self.femtos.get()
    }

    #[must_use]
    pub const fn hertz(self) -> u64 {
        1_000_000_000_000_000 / self.femtos.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonzero_periods() {
        const CLOCK: Period = Period::from_femtos(NonZero::new(10_000_000).unwrap());
        assert_eq!(CLOCK.femtos(), 10_000_000);
        assert_eq!(CLOCK.hertz(), 100_000_000);
        assert_eq!(
            Period::from_femtos(NonZero::<u64>::MIN).hertz(),
            1_000_000_000_000_000
        );
        assert_eq!(Period::from_femtos(NonZero::<u64>::MAX).femtos(), u64::MAX);
    }
}
