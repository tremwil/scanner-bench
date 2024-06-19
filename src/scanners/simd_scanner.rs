use std::io::Write;
use std::simd::cmp::SimdPartialEq;
use std::simd::{LaneCount, Simd, SupportedLaneCount};

use crate::pattern::Pattern;
use crate::scanner::Scanner;

fn to_simd<const N: usize>(slice: &[u8], default: u8) -> impl Iterator<Item = Simd<u8, N>> + '_
where
    LaneCount<N>: SupportedLaneCount,
{
    slice.chunks(N).map(move |c| {
        let mut chunk = Simd::splat(default);
        chunk.as_mut_array().as_mut_slice().write(c).unwrap();
        chunk
    })
}

/// Pattern which stores its bytes and mask as a pre-aligned SIMD vectors
/// for faster matching.
pub struct SimdPattern<const N: usize>
where
    LaneCount<N>: SupportedLaneCount,
{
    bytes: Vec<Simd<u8, N>>,
    mask: Vec<Simd<u8, N>>,
}

impl<const N: usize> Pattern for SimdPattern<N>
where
    LaneCount<N>: SupportedLaneCount,
{
    fn from_bytes_and_mask(bytes: &[u8], mask: &[u8]) -> Option<Self> {
        if bytes.len() != mask.len() {
            return None;
        }

        let mask: Vec<Simd<u8, N>> = to_simd(mask, 0).collect();
        // pre-apply mask to bytes
        let bytes = to_simd(bytes, 0)
            .zip(mask.iter())
            .map(|(b, m)| b & m)
            .collect();

        Some(Self { bytes, mask })
    }

    fn len(&self) -> usize {
        self.bytes.len() * N
    }

    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.bytes.as_ptr() as usize as *const u8, self.len()) }
    }

    fn mask(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.mask.as_ptr() as usize as *const u8, self.len()) }
    }

    unsafe fn matches_unchecked(&self, ptr: *const u8) -> bool {
        let ptr = ptr as usize as *const Simd<u8, N>;
        for i in 0..self.bytes.len() {
            if ptr.add(i).read_unaligned() & self.mask[i] != self.bytes[i] {
                return false;
            }
        }
        true
    }
}

/// Specialization of SimdPattern where the pattern fits inside a single SIMD register.
/// This makes matching very fast as the loop is removed entirely.
pub struct SmallSimdPattern<const N: usize>
where
    LaneCount<N>: SupportedLaneCount,
{
    bytes: Simd<u8, N>,
    mask: Simd<u8, N>,
}

impl<const N: usize> Pattern for SmallSimdPattern<N>
where
    LaneCount<N>: SupportedLaneCount,
{
    fn from_bytes_and_mask(bytes: &[u8], mask: &[u8]) -> Option<Self> {
        if bytes.len() != mask.len() || bytes.len() > N {
            return None;
        }

        let mask: Simd<u8, N> = to_simd(mask, 0).next().unwrap_or(Simd::splat(0));
        // pre-apply mask to bytes
        let bytes = to_simd(bytes, 0).next().unwrap_or(Simd::splat(0)) & mask;

        Some(Self { bytes, mask })
    }

    fn len(&self) -> usize {
        self.bytes.len() * N
    }

    fn bytes(&self) -> &[u8] {
        self.bytes.as_array()
    }

    fn mask(&self) -> &[u8] {
        self.mask.as_array()
    }

    unsafe fn matches_unchecked(&self, ptr: *const u8) -> bool {
        let ptr = ptr as usize as *const Simd<u8, N>;
        ptr.read_unaligned() & self.mask == self.bytes
    }
}

/// SIMD-accelerated scanner.
pub struct SimdScanner<const N: usize>
where
    LaneCount<N>: SupportedLaneCount,
{
    frequencies: [u8; 256],
}

impl<const N: usize> SimdScanner<N>
where
    LaneCount<N>: SupportedLaneCount,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_frequencies(frequencies: [u8; 256]) -> Self {
        Self { frequencies }
    }
}

impl<const N: usize> Default for SimdScanner<N>
where
    LaneCount<N>: SupportedLaneCount,
{
    fn default() -> Self {
        Self {
            frequencies: DEFAULT_FREQUENCIES,
        }
    }
}

impl<const N: usize> Scanner for SimdScanner<N>
where
    LaneCount<N>: SupportedLaneCount,
{
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        let range = haystack.as_ptr_range();

        // TODO: Do a naive scan on the unaligned portions of the region
        let lo_align = ((range.start as usize + N - 1) & !(N - 1)) as *const Simd<u8, N>;
        let hi_align = (range.end as usize - pat.len() & !(N - 1)) as *const Simd<u8, N>;
        let aligned_region: &[Simd<u8, N>] = unsafe {
            std::slice::from_raw_parts(lo_align, lo_align.offset_from(hi_align) as usize)
        };

        let (needle_index, needle) = pat
            .bytes()
            .iter()
            .copied()
            .enumerate()
            .min_by_key(|&(_, b)| self.frequencies[b as usize])?;

        let needle_splat: Simd<_, N> = Simd::splat(needle);

        for chunk in aligned_region {
            let mut eqmask = chunk.simd_eq(needle_splat).to_bitmask();
            while eqmask != 0 {
                let ofs = eqmask.trailing_zeros() as usize;
                unsafe {
                    let addr = chunk.as_array().as_ptr().add(ofs).sub(needle_index);
                    if pat.matches_unchecked(addr) {
                        return Some(addr.offset_from(range.start) as usize);
                    }
                }
                eqmask = eqmask & (eqmask - 1);
            }
        }

        None
    }

    fn find_all(&self, haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize> {
        todo!();
        return [].into_iter();
    }
}

const DEFAULT_FREQUENCIES: [u8; 256] = [
    0xFF, 0xFB, 0xF2, 0xEE, 0xEC, 0xE7, 0xDC, 0xC8, 0xED, 0xB7, 0xCC, 0xC0, 0xD3, 0xCD, 0x89, 0xFA,
    0xF3, 0xD6, 0x8D, 0x83, 0xC1, 0xAA, 0x7A, 0x72, 0xC6, 0x60, 0x3E, 0x2E, 0x98, 0x69, 0x39, 0x7C,
    0xEB, 0x76, 0x24, 0x34, 0xF9, 0x50, 0x04, 0x07, 0xE5, 0xAC, 0x53, 0x65, 0x9B, 0x4D, 0x6D, 0x5C,
    0xDA, 0x93, 0x7F, 0xCB, 0x92, 0x49, 0x43, 0x09, 0xBA, 0x8E, 0x1E, 0x91, 0x8A, 0x5B, 0x11, 0xA1,
    0xE8, 0xF5, 0x9E, 0xAD, 0xEF, 0xE6, 0x79, 0x7B, 0xFE, 0xE0, 0x1F, 0x54, 0xE4, 0xBD, 0x7D, 0x6A,
    0xDF, 0x67, 0x7E, 0xA4, 0xB6, 0xAF, 0x88, 0xA0, 0xC3, 0xA9, 0x26, 0x77, 0xD1, 0x71, 0x61, 0xC2,
    0x9A, 0xCA, 0x29, 0x9F, 0xD8, 0xE2, 0xD0, 0x6E, 0xB4, 0xB8, 0x25, 0x3C, 0xBF, 0x73, 0xB5, 0xCF,
    0xD4, 0x01, 0xCE, 0xBE, 0xF1, 0xDB, 0x52, 0x37, 0x9D, 0x63, 0x02, 0x6B, 0x80, 0x45, 0x2B, 0x95,
    0xE1, 0xC4, 0x36, 0xF0, 0xD5, 0xE3, 0x57, 0x9C, 0xB1, 0xF7, 0x82, 0xFC, 0x42, 0xF6, 0x18, 0x33,
    0xD2, 0x48, 0x05, 0x0F, 0x41, 0x1D, 0x03, 0x27, 0x70, 0x10, 0x00, 0x08, 0x55, 0x16, 0x2F, 0x0E,
    0x94, 0x35, 0x2C, 0x40, 0x6F, 0x3B, 0x1C, 0x28, 0x90, 0x68, 0x81, 0x4B, 0x56, 0x30, 0x2A, 0x3D,
    0x97, 0x17, 0x06, 0x13, 0x32, 0x0B, 0x5A, 0x75, 0xA5, 0x86, 0x78, 0x4F, 0x2D, 0x51, 0x46, 0x5F,
    0xE9, 0xDE, 0xA2, 0xDD, 0xC9, 0x4C, 0xAB, 0xBB, 0xC7, 0xB9, 0x74, 0x8F, 0xF8, 0x6C, 0x85, 0x8B,
    0xC5, 0x84, 0x8C, 0x66, 0x21, 0x23, 0x64, 0x59, 0xA3, 0x87, 0x44, 0x58, 0x3A, 0x0D, 0x12, 0x19,
    0xAE, 0x5E, 0x3F, 0x38, 0x31, 0x22, 0x0A, 0x14, 0xF4, 0xD9, 0x20, 0xB0, 0xB2, 0x1A, 0x0C, 0x15,
    0xB3, 0x47, 0x5D, 0xEA, 0x4A, 0x1B, 0x99, 0xBC, 0xD7, 0xA6, 0x62, 0x4E, 0xA8, 0x96, 0xA7, 0xFD,
];
