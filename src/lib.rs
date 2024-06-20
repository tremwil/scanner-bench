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
use scanners::simd_scanner::{SimdPattern, SimdScanner, SmallSimdPattern};
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
    std::process::Command::new("CMD")
        .args(["/C", "CLS"])
        .status()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    unsafe { setup_console()? };

    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()?;

    let file = std::fs::read(r"E:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe")?;
    let region = &file;

    fn bench<const N: usize, P: Pattern>(region: &[u8]) -> Result<(), Box<dyn Error>>
    where
        LaneCount<N>: SupportedLaneCount,
    {
        log::info!(
            "Begin tests for lane_count={N}, pattern={}",
            std::any::type_name::<P>()
        );

        let scanner = SimdScanner::<N>::new();
        let pattern = P::from_bytes_and_mask(BYTES, MASK).ok_or("Invalid pattern")?;

        let mut avg = Duration::ZERO;
        for _ in 0..1000 {
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
        avg /= 1000;

        log::info!("average for 1000 runs: {:.2?}", avg);

        Ok(())
    }

    bench::<64, SimdPattern<32>>(region)?;
    bench::<64, SimdPattern<64>>(region)?;
    bench::<64, SmallSimdPattern<32>>(region)?;
    bench::<64, SmallSimdPattern<64>>(region)?;

    bench::<32, SimdPattern<32>>(region)?;
    bench::<32, SmallSimdPattern<32>>(region)?;

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
