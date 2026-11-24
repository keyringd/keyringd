//! The UI test fixture. Built only with the `test-support` feature and
//! never part of a distributed build.

fn main() -> anyhow::Result<()> {
    // SAFETY: this is before the application starts any worker threads.
    unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "1") };
    keyringd_macos::run_ui_test_app()
}
