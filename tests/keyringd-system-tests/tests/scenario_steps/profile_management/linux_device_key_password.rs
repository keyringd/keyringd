use anyhow::{Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profiles::linux::LinuxProfileActionsMenu;
use keyringd_system_tests::environment::{
    SystemTestEnvironment, linux::LinuxSystemTestEnvironment,
};

pub fn assert_menu_actions(menu: &LinuxProfileActionsMenu<'_>, protected: bool) -> Result<()> {
    for (label, expected) in [
        (
            "Enable local authentication…",
            if protected { None } else { Some(true) },
        ),
        (
            "Change Device Key Password…",
            if protected { Some(true) } else { None },
        ),
        (
            "Disable local authentication…",
            if protected { Some(true) } else { None },
        ),
    ] {
        let actual = menu.action_enabled(label)?;
        ensure!(
            actual == expected,
            "{label}: expected {expected:?}, observed {actual:?}"
        );
    }
    Ok(())
}
fn assert_menu(
    environment: &mut LinuxSystemTestEnvironment,
    name: &str,
    protected: bool,
) -> Result<()> {
    let profiles = environment.application().profiles_window()?;
    let mut menu = profiles.open_profile_actions_menu(name)?;
    assert_menu_actions(&menu, protected)?;
    menu.close()
}
pub fn enable(
    environment: &mut LinuxSystemTestEnvironment,
    name: &str,
    password: &str,
) -> Result<()> {
    {
        let profiles = environment.application().profiles_window()?;
        let mut menu = profiles.open_profile_actions_menu(name)?;
        let window = menu.open_password_protection()?;
        window.enter_password("New password", password)?;
        window.enter_password("Confirm new password", password)?;
        window.submit()?;
        window.wait_closed()?;
        window.wait_for_notification("Device Key local authentication enabled")?;
    }
    assert_menu(environment, name, true)
}
pub fn change(
    environment: &mut LinuxSystemTestEnvironment,
    name: &str,
    current: &str,
    password: &str,
) -> Result<()> {
    {
        let profiles = environment.application().profiles_window()?;
        let mut menu = profiles.open_profile_actions_menu(name)?;
        let window = menu.open_password_change()?;
        window.enter_password("Current password", current)?;
        window.enter_password("New password", password)?;
        window.enter_password("Confirm new password", password)?;
        window.submit()?;
        window.wait_closed()?;
        window.wait_for_notification("Device Key local authentication updated")?;
    }
    assert_menu(environment, name, true)
}
pub fn disable(
    environment: &mut LinuxSystemTestEnvironment,
    name: &str,
    password: &str,
) -> Result<()> {
    {
        let profiles = environment.application().profiles_window()?;
        let mut menu = profiles.open_profile_actions_menu(name)?;
        let window = menu.open_password_removal()?;
        window.enter_password("Current password", password)?;
        window.submit()?;
        window.wait_closed()?;
        window.wait_for_notification("Device Key local authentication disabled")?;
    }
    assert_menu(environment, name, false)
}
