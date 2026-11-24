use anyhow::Result;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::environment::SystemTestEnvironment;
use std::path::PathBuf;

pub fn read_qr(environment: &mut impl SystemTestEnvironment) -> Result<(String, PathBuf)> {
    let (qr, image) = environment
        .application()
        .qr_prompt_window()?
        .read_hybrid_qr()?;
    assert!(qr.starts_with("FIDO:/"), "invalid Hybrid QR: {qr}");
    Ok((qr, image))
}
