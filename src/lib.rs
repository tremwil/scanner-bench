#![feature(portable_simd)]
#![feature(core_intrinsics)]

use std::{
    error::Error,
    fs::File,
    simd::{LaneCount, SupportedLaneCount},
    time::{Duration, Instant},
};

use pattern::{BasicPattern, Pattern};
use pelite::pe::{Pe, PeObject, PeView};
use scanner::Scanner;
use scanners::{
    multi_needle_simd::MultiNeedleSimd,
    simd_scanner::{SimdPattern, SimdScanner, SmallSimdPattern},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HINSTANCE,
        System::{
            Console::{AllocConsole, GetConsoleWindow},
            LibraryLoader::{
                DisableThreadLibraryCalls, FreeLibraryAndExitThread, GetModuleHandleW,
            },
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
};

mod pattern;
mod scanner;
mod scanners;

const BYTES: &[u8] = &[
    0x48, 0x89, 0x5c, 0x24, 0x0, 0x48, 0x89, 0x74, 0x24, 0x0, 0x57, 0x48, 0x83, 0xec, 0x0, 0x48,
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

        let mut avg = Duration::ZERO;
        const N_TESTS: u32 = 1000;
        for _ in 0..N_TESTS {
            let start = Instant::now();

            let opt_offset = scanner.find_one(region, &pattern);
            let elapsed = start.elapsed();

            if let Some(ofs) = opt_offset {
                log::trace!(
                    "time={:.2?} offset={:x} address={:x}",
                    elapsed,
                    ofs,
                    region.as_ptr() as usize + ofs
                );
            }
            log::trace!("time={:.2?}", elapsed);

            avg += elapsed;
        }
        avg /= N_TESTS;

        log::info!("average for {N_TESTS} runs: {:.2?}", avg);

        Ok(())
    }

    bench::<SimdScanner<32>, SmallSimdPattern<32>>(region)?;
    bench::<SimdScanner<64>, SmallSimdPattern<32>>(region)?;
    bench::<MultiNeedleSimd<32, 2>, SmallSimdPattern<32>>(region)?;
    bench::<MultiNeedleSimd<64, 2>, SmallSimdPattern<32>>(region)?;

    Ok(())
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    h_inst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const (),
) -> i32 {
    if fdw_reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(h_inst_dll).ok();

        let _ = std::thread::spawn(move || {
            let exit_code = match std::panic::catch_unwind(main) {
                Err(e) => {
                    println!("scanner-bench panicked in main: {:#?}", e);
                    1
                }
                Ok(Err(e)) => {
                    println!("error during scanner-bench main: {:#?}", e);
                    1
                }
                Ok(_) => 0,
            };

            log::debug!("scanner-bench unloading");
            FreeLibraryAndExitThread(h_inst_dll, exit_code);
        });
    }
    1
}
