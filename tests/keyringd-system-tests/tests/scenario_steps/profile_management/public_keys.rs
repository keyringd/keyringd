use anyhow::{Result, anyhow};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profiles::{
    ProfilesWindowDriver, TrustEntries, TrustEntriesWindowDriver,
};
use keyringd_system_tests::environment::SystemTestEnvironment;

pub fn copy_ca(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<String> {
    with_window(environment, name, |window| window.copy_ca_trust_entry())
}

pub fn copy_device_key(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<String> {
    with_window(environment, name, |window| {
        window.copy_device_key_trust_entry()
    })
}

pub fn copy_passkey(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<String> {
    with_window(environment, name, |window| {
        window.copy_passkey_trust_entry()
    })
}

pub fn read(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<TrustEntries> {
    with_window(environment, name, |window| {
        let [ca, device, passkey] =
            window
                .displayed_entries()?
                .try_into()
                .map_err(|entries: Vec<String>| {
                    anyhow!("expected three public keys, found {}", entries.len())
                })?;
        Ok(TrustEntries {
            ca,
            device,
            passkey,
        })
    })
}

fn with_window<T>(
    environment: &mut impl SystemTestEnvironment,
    name: &str,
    read: impl FnOnce(&dyn TrustEntriesWindowDriver) -> Result<T>,
) -> Result<T> {
    let mut profiles = environment.application().profiles_window()?;
    let mut window = profiles.open_trust_entries(name)?;
    let value = read(&window)?;
    window.close()?;
    Ok(value)
}
