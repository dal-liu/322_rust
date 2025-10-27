#[derive(Debug)]
pub struct BitVector {
    vec: Vec<u64>,
    len: usize,
}

impl BitVector {
    const BITS_PER_WORD: usize = 64;

    pub fn with_capacity(capacity: usize) -> Self {
        let num_words = (capacity + Self::BITS_PER_WORD - 1) / Self::BITS_PER_WORD;
        Self {
            vec: vec![0; num_words],
            len: capacity,
        }
    }
}
