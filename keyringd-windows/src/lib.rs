mod app;
mod system;

pub use app::WindowsEntryPoint;
pub use app::handler;

/// Runs the product UI and workflows with isolated test services.
#[cfg(feature = "test-support")]
pub fn run_ui_test_app() -> anyhow::Result<()> {
    app::ui_test_support::run()
}
pub(crate) use app::WindowsError;

#[cfg(test)]
mod tests;
