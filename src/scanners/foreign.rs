use crate::pattern::Pattern;
use crate::scanner::Scanner;

#[link(name = "c_scanners_ffi", kind = "static")]
extern "C" {
    fn scan_mem_simd(
        region: *const u8,
        region_len: usize,
        bytes: *const u8,
        mask: *const u8,
        len: usize,
    ) -> usize;

    fn scan_pattern16(
        region: *const u8,
        region_len: usize,
        bytes: *const u8,
        mask: *const u8,
        len: usize,
    ) -> usize;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MemScanner;

impl Scanner for MemScanner {
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        let res = unsafe {
            scan_mem_simd(
                haystack.as_ptr(),
                haystack.len(),
                pat.bytes().as_ptr(),
                pat.mask().as_ptr(),
                pat.len(),
            )
        };
        (res != 0).then(|| res - haystack.as_ptr() as usize)
    }

    fn find_all(&self, haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize> {
        todo!();
        [].into_iter()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Pat16Scanner;

impl Scanner for Pat16Scanner {
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        let res = unsafe {
            scan_pattern16(
                haystack.as_ptr(),
                haystack.len(),
                pat.bytes().as_ptr(),
                pat.mask().as_ptr(),
                pat.len(),
            )
        };
        (res != 0).then(|| res - haystack.as_ptr() as usize)
    }

    fn find_all(&self, haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize> {
        todo!();
        [].into_iter()
    }
}
