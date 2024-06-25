#![feature(portable_simd)]
#![feature(core_intrinsics)]

use core::arch::x86_64::_rdtsc;
use std::{
    error::Error,
    time::{Duration, Instant},
};

use pattern::Pattern;
use scanner::Scanner;
use scanners::{
    foreign::{MemScanner, Pat16Scanner},
    multi_needle_simd::MultiNeedleSimd,
    simd::{SimdScanner, SmallSimdPattern},
};
use windows::Win32::System::{
    Console::{AllocConsole, GetConsoleWindow},
    Threading::{
        GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadAffinityMask,
        SetThreadPriority, REALTIME_PRIORITY_CLASS, THREAD_PRIORITY_TIME_CRITICAL,
    },
};

mod pattern;
mod scanner;
mod scanners;

const BYTES: &[u8] = &[
    0x48, 0x89, 0x5c, 0x24, 0x0, 0x48, 0x88, 0x74, 0x24, 0x0, 0x57, 0x48, 0x83, 0xec, 0x0, 0x48,
    0x8b, 0x1, 0x48, 0x8b, 0xf9, 0x32, 0xdb,
];
const MASK: &[u8] = &[
    0xff, 0xff, 0xff, 0xff, 0x0, 0xff, 0xff, 0xff, 0xff, 0x0, 0xff, 0xff, 0xff, 0xff, 0x0, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

unsafe fn setup_console() -> Result<(), Box<dyn Error>> {
    if GetConsoleWindow().0 == 0 && AllocConsole().is_err() {
        return Err("Failed to allocate console".into());
    }
    std::process::Command::new("CMD").args(["/C", "CLS"]).status()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    unsafe { setup_console()? };

    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Info).init()?;

    let file = std::fs::read(r"E:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe")?;
    let region = &file;

    fn bench<S: Scanner + Default, P: Pattern>(region: &[u8]) -> Result<(), Box<dyn Error>> {
        log::info!(
            "Begin tests for scanner={}, pattern={}",
            std::any::type_name::<S>().split(':').last().unwrap(),
            std::any::type_name::<P>().split(':').last().unwrap()
        );

        let scanner = S::default();
        let pattern = P::from_bytes_and_mask(BYTES, MASK).ok_or("Invalid pattern")?;

        let mut first_ofs = None;
        let mut avg = Duration::ZERO;
        let mut avg_clk = 0u64;
        const N_TESTS: u32 = 1000;

        for _ in 0..N_TESTS {
            let start = Instant::now();
            let start_clk = unsafe { _rdtsc() };

            let offset = scanner.find_one(region, &pattern).unwrap_or(0);
            let elasped_clk = unsafe { _rdtsc() } - start_clk;
            let elapsed = start.elapsed();

            if *first_ofs.get_or_insert(offset) != offset {
                return Err(format!("Scanners do not agree on offset: {:x}", offset).into());
            }

            avg += elapsed;
            avg_clk += elasped_clk;
        }
        avg /= N_TESTS;
        avg_clk /= N_TESTS as u64;

        let bpc = region.len() as f64 / avg_clk as f64;

        log::info!(
            "average for {N_TESTS} runs: {:.2?} ({} cycles = {:.3} b/c)",
            avg,
            avg_clk,
            bpc
        );

        Ok(())
    }

    unsafe {
        SetPriorityClass(GetCurrentProcess(), REALTIME_PRIORITY_CLASS)?;
        SetThreadAffinityMask(GetCurrentThread(), 1);
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL)?;
    }

    bench::<MemScanner, SmallSimdPattern<32>>(region)?;
    // bench::<Pat16Scanner, SmallSimdPattern<32>>(region)?;
    bench::<SimdScanner<32>, SmallSimdPattern<32>>(region)?;
    bench::<SimdScanner<64>, SmallSimdPattern<32>>(region)?;
    bench::<MultiNeedleSimd<32, 1>, SmallSimdPattern<32>>(region)?;
    bench::<MultiNeedleSimd<32, 2>, SmallSimdPattern<32>>(region)?;

    Ok(())
}
