#[derive(Debug, Clone, Copy)]
pub struct Period {
    femtos: u64,
}

impl Period {
    #[must_use]
    pub const fn from_femtos(femtos: u64) -> Self {
        Self { femtos }
    }

    #[must_use]
    pub const fn femtos(self) -> u64 {
        self.femtos
    }

    #[must_use]
    pub const fn hertz(self) -> u64 {
        1_000_000_000_000_000 / self.femtos
    }
}
