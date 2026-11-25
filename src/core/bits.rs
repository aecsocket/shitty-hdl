use smallvec::SmallVec;

pub struct Bits<const N: usize>(SmallVec<u64, 1>);

impl<const N: usize> Bits<N> {
    //     pub fn from_word(word: u64) -> Self {
    //         Self(SmallVec::from_buf_and_len([word, 0], 1))
    //     }
    //
    //     pub fn from_words(words: impl IntoIterator<Item = u64>) -> Self {
    //         Self(words.into_iter().collect())
    //     }
    //
    //     pub fn into_words<const N: usize>(self) -> Option<[u64; N]> {
    //         <[u64; N]>::try_from(self.0).ok()
    //     }
}
