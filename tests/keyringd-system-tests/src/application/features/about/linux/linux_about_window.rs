use super::LinuxLicenseWindow;
use crate::application::features::about::{AboutApplicationInfo, AboutWindowDriver};
use crate::application::features::navigation::linux::MainMenu;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use keyringd_ui_core::about_window::LicenseDocument;
use keyringd_ui_core::text as ui_text;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxAboutWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl LinuxAboutWindow<'_> {
    async fn window(&self) -> Result<UiDriver> {
        let application = self.session.application();
        let window = application
            .wait_for_window(ui_text::ABOUT_TITLE, ELEMENT_TIMEOUT)
            .await?;
        Ok(application.within(&window))
    }

    fn open_license(&self, document: LicenseDocument) -> Result<LinuxLicenseWindow<'_>> {
        let session = self.runtime.block_on(async {
            let session = self.window().await?;
            let label = match document {
                LicenseDocument::Application => ui_text::ABOUT_LICENSE,
                LicenseDocument::ThirdParty => ui_text::ABOUT_THIRD_PARTY_LICENSES,
            };
            let open = session
                .wait_for_enabled_action(label, ELEMENT_TIMEOUT)
                .await?;
            session.click(&open).await?;
            let application = session.application();
            let window = application
                .wait_for_window(document.title(), ELEMENT_TIMEOUT)
                .await?;
            let session = application.within(&window);
            session
                .wait_for_visible_name(ui_text::ABOUT_LICENSE_ACCESSIBILITY, ELEMENT_TIMEOUT)
                .await?;
            Ok::<_, anyhow::Error>(session)
        })?;
        Ok(LinuxLicenseWindow {
            runtime: self.runtime,
            session,
            document,
        })
    }
}

impl AboutWindowDriver for LinuxAboutWindow<'_> {
    type ApplicationLicense<'a>
        = LinuxLicenseWindow<'a>
    where
        Self: 'a;
    type ThirdPartyLicensesWindow<'a>
        = LinuxLicenseWindow<'a>
    where
        Self: 'a;

    fn open(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            MainMenu::open(self.session.clone())
                .await?
                .open_about()
                .await?;
            self.window().await?;
            Ok(())
        })
    }
    fn application_info(&self) -> Result<AboutApplicationInfo> {
        self.runtime.block_on(async {
            Ok(AboutApplicationInfo {
                texts: self
                    .window()
                    .await?
                    .visible_names_with_role(atspi::Role::Label)
                    .await?,
                enabled_actions: Vec::new(),
            })
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime
            .block_on(async { self.window().await?.capture().await })
    }
    fn open_application_license(&mut self) -> Result<Self::ApplicationLicense<'_>> {
        self.open_license(LicenseDocument::Application)
    }
    fn open_third_party_licenses(&mut self) -> Result<Self::ThirdPartyLicensesWindow<'_>> {
        self.open_license(LicenseDocument::ThirdParty)
    }
    fn close(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.window().await?;
            let close = session
                .wait_for_visible_action("Close", ELEMENT_TIMEOUT)
                .await?;
            session.click(&close).await?;
            session
                .application()
                .wait_for_visible_absence(ui_text::ABOUT_TITLE, ELEMENT_TIMEOUT)
                .await
        })
    }
}
