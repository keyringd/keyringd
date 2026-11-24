fn main() -> anyhow::Result<()> {
    // SAFETY: this is before the application starts any worker threads.
    unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "1") };
    keyringd_macos::run()
}
