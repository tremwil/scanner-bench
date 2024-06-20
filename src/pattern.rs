pub trait Pattern: Sized {
    fn from_bytes_and_mask(bytes: &[u8], mask: &[u8]) -> Option<Self>;

    fn bytes(&self) -> &[u8];
    fn mask(&self) -> &[u8];

    fn len(&self) -> usize {
        self.bytes().len()
    }

    fn from_string(string: impl AsRef<str>) -> Option<Self> {
        todo!()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mask = vec![0xff; bytes.len()];
        Self::from_bytes_and_mask(bytes, mask.as_slice())
    }

    fn from_pattern(pat: impl Pattern) -> Self {
        Self::from_bytes_and_mask(pat.bytes(), pat.mask()).unwrap()
    }

    #[inline(always)]
    unsafe fn matches_unchecked(&self, ptr: *const u8) -> bool {
        for i in 0..self.bytes().len() {
            if ptr.add(i).read() & self.mask()[i] != self.bytes()[i] {
                return false;
            }
        }
        true
    }
}

pub struct BasicPattern {
    bytes: Vec<u8>,
    mask: Vec<u8>,
}

impl Pattern for BasicPattern {
    fn from_bytes_and_mask(bytes: &[u8], mask: &[u8]) -> Option<Self> {
        (bytes.len() == mask.len()).then(|| Self {
            // pre-apply the mask to the bytes
            bytes: bytes.iter().zip(mask).map(|(&b, &m)| b & m).collect(),
            mask: mask.to_vec(),
        })
    }

    fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    fn mask(&self) -> &[u8] {
        self.mask.as_slice()
    }
}
