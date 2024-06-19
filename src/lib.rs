#![feature(portable_simd)]

use std::error::Error;

use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        Console::{AllocConsole, GetConsoleWindow},
        LibraryLoader::{DisableThreadLibraryCalls, FreeLibraryAndExitThread},
        SystemServices::DLL_PROCESS_ATTACH,
    },
};

mod pattern;
mod scanner;
mod scanners;

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
