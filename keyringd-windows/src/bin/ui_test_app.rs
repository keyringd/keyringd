fn main() -> anyhow::Result<()> {
    // SAFETY: environment mutation is thread-safe on Windows.
    unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "1") };
    keyringd_windows::run_ui_test_app()
}
