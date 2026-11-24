fn main() {
    // SAFETY: this is before the application starts any worker threads.
    unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "1") };
    if keyringd_linux::run().is_err() {
        std::process::exit(1);
    }
}
