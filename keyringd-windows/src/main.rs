#![windows_subsystem = "windows"]

fn main() {
    // SAFETY: environment mutation is thread-safe on Windows.
    unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "1") };
    if keyringd_windows::WindowsEntryPoint::run().is_err() {
        std::process::exit(1);
    }
}
