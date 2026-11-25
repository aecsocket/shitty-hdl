use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PackedBits {
    width: u32,
    words: Vec<u64>,
}

impl PackedBits {
    pub fn zero(width: u32) -> Self {
        Self::from_words(width, Vec::new())
    }
    pub fn from_words(width: u32, mut words: Vec<u64>) -> Self {
        words.resize((width as usize).div_ceil(64), 0);
        if width % 64 != 0
            && let Some(last) = words.last_mut()
        {
            *last &= (1u64 << (width % 64)) - 1;
        }
        Self { width, words }
    }
    pub fn from_u64(width: u32, value: u64) -> Self {
        Self::from_words(width, vec![value])
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn words(&self) -> &[u64] {
        &self.words
    }
    pub fn bit(&self, index: u32) -> bool {
        index < self.width && (self.words[index as usize / 64] >> (index % 64)) & 1 != 0
    }
    pub fn set_bit(&mut self, index: u32, value: bool) {
        if index >= self.width {
            return;
        }
        let mask = 1u64 << (index % 64);
        let word = &mut self.words[index as usize / 64];
        if value {
            *word |= mask;
        } else {
            *word &= !mask;
        }
    }
    pub fn slice(&self, offset: u32, width: u32) -> Self {
        let mut out = Self::zero(width);
        for b in 0..width {
            out.set_bit(b, offset.checked_add(b).is_some_and(|i| self.bit(i)));
        }
        out
    }
    pub fn replace(&mut self, offset: u32, value: &Self) {
        for b in 0..value.width {
            if let Some(i) = offset.checked_add(b) {
                self.set_bit(i, value.bit(b));
            }
        }
    }
    pub(crate) fn not(&self) -> Self {
        Self::from_words(self.width, self.words.iter().map(|w| !w).collect())
    }
    pub(crate) fn add(&self, rhs: &Self) -> Self {
        let mut carry = false;
        let words = self
            .words
            .iter()
            .zip(&rhs.words)
            .map(|(&a, &b)| {
                let (v, c1) = a.overflowing_add(b);
                let (v, c2) = v.overflowing_add(carry as u64);
                carry = c1 || c2;
                v
            })
            .collect();
        Self::from_words(self.width, words)
    }
    pub(crate) fn neg(&self) -> Self {
        self.not().add(&Self::from_u64(self.width, 1))
    }
    pub(crate) fn mul(&self, rhs: &Self) -> Self {
        let n = self.words.len();
        let mut words = vec![0u64; n];
        for i in 0..n {
            let mut carry = 0u128;
            for j in 0..n - i {
                let v = self.words[i] as u128 * rhs.words[j] as u128 + words[i + j] as u128 + carry;
                words[i + j] = v as u64;
                carry = v >> 64;
            }
        }
        Self::from_words(self.width, words)
    }
    pub(crate) fn bitop(&self, rhs: &Self, op: fn(u64, u64) -> u64) -> Self {
        Self::from_words(
            self.width,
            self.words
                .iter()
                .zip(&rhs.words)
                .map(|(&a, &b)| op(a, b))
                .collect(),
        )
    }
    pub(crate) fn cmp(&self, rhs: &Self, signed: bool) -> Ordering {
        if signed && self.width != 0 {
            let a = self.bit(self.width - 1);
            let b = rhs.bit(rhs.width - 1);
            if a != b {
                return if a { Ordering::Less } else { Ordering::Greater };
            }
        }
        self.words.iter().rev().cmp(rhs.words.iter().rev())
    }
    pub(crate) fn extend(&self, width: u32, sign: bool) -> Self {
        let mut out = Self::from_words(width, self.words.clone());
        if sign && self.width > 0 && self.bit(self.width - 1) {
            for b in self.width..width {
                out.set_bit(b, true);
            }
        }
        out
    }
    pub(crate) fn shift(&self, count: &Self, left: bool, arithmetic: bool) -> Self {
        let huge = count.words.iter().skip(1).any(|&v| v != 0);
        let amount = if huge {
            u64::MAX
        } else {
            count.words.first().copied().unwrap_or(0)
        };
        let sign = arithmetic && self.width > 0 && self.bit(self.width - 1);
        let mut out = Self::zero(self.width);
        for b in 0..self.width {
            let value = if left {
                if amount <= b as u64 {
                    self.bit(b - amount as u32)
                } else {
                    false
                }
            } else if amount < self.width as u64 && (b as u64 + amount) < self.width as u64 {
                self.bit(b + amount as u32)
            } else {
                sign
            };
            out.set_bit(b, value);
        }
        out
    }
}
