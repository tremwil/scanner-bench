use std::ffi::{CStr, CString};

use windows::core::PCSTR;
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

use crate::pattern::Pattern;
use crate::scanner::Scanner;

type ScannerFn = unsafe extern "C" fn(*const u8, usize, *const u8, *const u8, usize) -> usize;

#[derive(Debug)]
pub struct ForeignScanner {
    hmod: HMODULE,
    fun: ScannerFn,
}

impl Drop for ForeignScanner {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.hmod).ok();
        }
    }
}

impl ForeignScanner {
    pub fn new(mod_name: &str, fun_name: &str) -> windows::core::Result<Self> {
        unsafe {
            let mod_name = CString::new(mod_name).unwrap();
            let fun_name = CString::new(fun_name).unwrap();

            let hmod = LoadLibraryA(PCSTR(mod_name.as_ptr() as *const u8))?;
            let fun =
                std::mem::transmute(GetProcAddress(hmod, PCSTR(fun_name.as_ptr() as *const u8)));
            Ok(Self { hmod, fun })
        }
    }
}

impl Scanner for ForeignScanner {
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        let res = unsafe {
            (self.fun)(
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

pub struct MemScanner(ForeignScanner);

impl Default for MemScanner {
    fn default() -> Self {
        Self(ForeignScanner::new("c_scanners_ffi.dll", "scan_mem_simd").unwrap())
    }
}

impl Scanner for MemScanner {
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        self.0.find_one(haystack, pat)
    }

    fn find_all(&self, haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize> {
        self.0.find_all(haystack, pat)
    }
}

pub struct Pat16Scanner(ForeignScanner);

impl Default for Pat16Scanner {
    fn default() -> Self {
        Self(ForeignScanner::new("c_scanners_ffi.dll", "scan_pattern16").unwrap())
    }
}

impl Scanner for Pat16Scanner {
    fn find_one(&self, haystack: &[u8], pat: &impl Pattern) -> Option<usize> {
        self.0.find_one(haystack, pat)
    }

    fn find_all(&self, haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize> {
        self.0.find_all(haystack, pat)
    }
}
