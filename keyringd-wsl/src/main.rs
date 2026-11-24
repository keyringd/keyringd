mod app;
mod system;

#[tokio::main]
async fn main() {
    if let Err(error) = app::WslEntryPoint::run().await {
        tracing::error!(?error, "keyringd WSL proxy stopped with an error");
        std::process::exit(1);
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
