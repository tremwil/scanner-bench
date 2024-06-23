use core::intrinsics::{prefetch_read_data, unlikely};
use std::simd::cmp::SimdPartialEq;
use std::simd::{LaneCount, Simd, SupportedLaneCount};

use itertools::Itertools;

use super::simd::DEFAULT_FREQUENCIES;
use crate::main;
use crate::pattern::Pattern;
use crate::scanner::Scanner;

/// SIMD-accelerated scanner using an `N`-byte matching algorithm.
pub struct MultiNeedleSimd<const L: usize, const N: usize>
where
    LaneCount<L>: SupportedLaneCount,
{
    frequencies: [u8; 256],
}

impl<const L: usize, const N: usize> MultiNeedleSimd<L, N>
where
    LaneCount<L>: SupportedLaneCount,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_frequencies(frequencies: [u8; 256]) -> Self {
        Self { frequencies }
    }
}

impl<const L: usize, const N: usize> Default for MultiNeedleSimd<L, N>
where
    LaneCount<L>: SupportedLaneCount,
{
    fn default() -> Self {
        Self {
            frequencies: DEFAULT_FREQUENCIES,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Needle {
    index: usize,
    value: u8,
}

fn find_needles<const N: usize>(pat: &impl Pattern, freqs: &[u8; 256]) -> Option<[Needle; N]> {
    let mut candidates: [Option<Needle>; N] = [None; N];

    for (i, &b) in pat.bytes().iter().enumerate().filter(|p| pat.mask()[p.0] == 0xff) {
        if candidates.iter().filter_map(|c| c.map(|c| c.value)).contains(&b) {
            continue;
        }
        for j in (0..candidates.len()).rev() {
            if candidates[j]
                .map(|c| freqs[b as usize] < freqs[c.value as usize])
                .unwrap_or(true)
            {
                if j + 1 == candidates.len() {
                    candidates[j] = Some(Needle { index: i, value: b });
                }
                else {
                    candidates.swap(j, j + 1);
                }
            }
        }
    }

    if candidates.iter().any(|c| c.is_none()) {
        return None;
    }
    let mut candidates = candidates.map(Option::unwrap);
    candidates.sort_by_key(|c| c.index);
    Some(candidates)
}

impl<const L: usize, const N: usize> Scanner for MultiNeedleSimd<L, N>
where
    LaneCount<L>: SupportedLaneCount,
{
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        let range = haystack.as_ptr_range();

        // TODO: Do a naive scan on the unaligned portions of the region
        let lo_align = ((range.start as usize + L - 1) & !(L - 1)) as *const Simd<u8, L>;
        let hi_align = (range.end as usize - pat.len() & !(L - 1)) as *const Simd<u8, L>;
        let aligned_region: &[Simd<u8, L>] = unsafe {
            std::slice::from_raw_parts(lo_align, hi_align.offset_from(lo_align) as usize)
        };

        let needles = find_needles::<N>(pat, &self.frequencies)?;
        let main_offset = needles[0].index;
        let needles = needles.map(|n| Needle {
            index: n.index - main_offset,
            value: n.value,
        });

        let masks = {
            let mut arr = [Simd::<u8, L>::splat(0); N];
            arr.iter_mut()
                .zip(needles)
                .for_each(|(s, n)| *s = Simd::<u8, L>::splat(n.value));
            arr
        };

        for chunk in aligned_region {
            unsafe { prefetch_read_data(chunk.as_array().as_ptr().add(L * 64), 3) };

            let chunk_ptr = chunk.as_array().as_ptr();
            let mut bitmask = (1..N)
                .fold(chunk.simd_eq(masks[0]), |m, i| unsafe {
                    let simd_ptr = chunk_ptr.add(needles[i].index) as usize as *const Simd<u8, L>;
                    m & simd_ptr.read_unaligned().simd_eq(masks[i])
                })
                .to_bitmask();

            while unlikely(bitmask != 0) {
                let ofs = bitmask.trailing_zeros() as usize;
                unsafe {
                    let addr = chunk_ptr.add(ofs).sub(main_offset);
                    if unlikely(pat.matches_unchecked(addr)) {
                        return Some(addr.offset_from(range.start) as usize);
                    }
                }
                bitmask = bitmask & (bitmask - 1);
            }
        }

        None
    }

    fn find_all(&self, haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize> {
        todo!();
        return [].into_iter();
    }
}
