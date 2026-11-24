use keyringd_ui_core::text as ui_text;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetLastActivePopup, IsWindow, SetForegroundWindow};
use windows_reactor::{
    AccessibilityExt, BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment,
    HyperlinkButton, Image, LayoutExt, PaddingExt, TextStyleExt, ThemeRef, Thickness, UiMarshaller,
    VerticalAlignment, border, button, grid, hstack, text_block, vstack,
};

use crate::app::ui::AuxiliaryWindow;
use crate::app::ui::third_party_licenses_window::ThirdPartyLicensesWindow;
use keyringd_ui_core::about_window::LicenseDocument;

const WEBSITE: &str = ui_text::ABOUT_WEBSITE_URL;
const ISSUES: &str = ui_text::ABOUT_ISSUES_URL;

#[derive(Clone)]
pub(crate) struct AboutWindow {
    dispatcher: UiMarshaller,
    handle: Arc<AtomicUsize>,
}

impl AboutWindow {
    pub(crate) fn new(dispatcher: UiMarshaller) -> Self {
        Self {
            dispatcher,
            handle: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub(crate) fn open(&self) -> anyhow::Result<()> {
        let handle = Arc::clone(&self.handle);
        anyhow::ensure!(
            self.dispatcher.dispatch(move || {
                if let Err(error) = Self::show(handle) {
                    tracing::error!(%error, "could not show About");
                }
            }),
            "About UI is unavailable"
        );
        Ok(())
    }

    fn show(handle: Arc<AtomicUsize>) -> anyhow::Result<()> {
        let existing = HWND(handle.load(Ordering::Acquire) as *mut _);
        if !existing.0.is_null() && unsafe { IsWindow(Some(existing)) }.as_bool() {
            // If a license is open, bring that modal child forward instead.
            unsafe {
                let _ = SetForegroundWindow(GetLastActivePopup(existing));
            }
            return Ok(());
        }

        // Embed the existing product artwork and retain its SVG file for the
        // lifetime of the window, so WinUI can load it without installed assets.
        let icon = Rc::new(tempfile::Builder::new().suffix(".svg").tempfile()?);
        std::fs::write(
            icon.path(),
            include_str!("../../../../../packaging/windows/keyringd-icon.svg"),
        )?;
        let icon_uri = format!("file:///{}", icon.path().display()).replace('\\', "/");
        let tracked = Arc::clone(&handle);
        let window = AuxiliaryWindow::open_root_tracked(
            ui_text::ABOUT_TITLE,
            (520.0, 330.0),
            move |cx, window| {
                // Keep the extracted artwork alive while the render closure lives.
                let _ = &icon;
                let tracked = Arc::clone(&tracked);
                cx.use_effect_with_cleanup((), move || {
                    Some(move || tracked.store(0, Ordering::Release))
                });
                Self::render(window, &icon_uri)
            },
        )?;
        handle.store(window.hwnd(), Ordering::Release);
        unsafe {
            let _ = SetForegroundWindow(HWND(window.hwnd() as *mut _));
        }
        Ok(())
    }

    fn render(window: AuxiliaryWindow, icon_uri: &str) -> Element {
        let application_license = window.clone();
        let third_party_licenses = window.clone();
        let close = window.clone();
        let details = vstack((
            vstack((
                text_block("Keyringd").font_size(28.0).semibold(),
                text_block(keyringd_ui_core::about_window::version(env!(
                    "CARGO_PKG_VERSION"
                )))
                .foreground(ThemeRef::SecondaryText),
            ))
            .spacing(4.0),
            vstack((
                text_block(ui_text::ABOUT_MAINTAINERS).foreground(ThemeRef::SecondaryText),
                text_block(ui_text::ABOUT_LICENSE_NAME).foreground(ThemeRef::SecondaryText),
            ))
            .spacing(4.0),
            hstack((
                HyperlinkButton::new(ui_text::ABOUT_WEBSITE)
                    .navigate_uri(WEBSITE)
                    .padding(0.0),
                HyperlinkButton::new(ui_text::ABOUT_REPORT_ISSUE)
                    .navigate_uri(ISSUES)
                    .padding(0.0),
            ))
            .spacing(20.0),
        ))
        .spacing(20.0)
        .grid_column(1);
        let body = grid((
            Image::new(icon_uri)
                .width(80.0)
                .height(80.0)
                .vertical_alignment(VerticalAlignment::Top)
                .automation_name("Keyringd icon"),
            details,
        ))
        .columns([GridLength::Auto, GridLength::STAR])
        .column_spacing(24.0)
        .padding(32.0);
        let documents = hstack((
            button(ui_text::ABOUT_LICENSE).on_click(move || {
                Self::open_license(&application_license, LicenseDocument::Application);
            }),
            button(ui_text::ABOUT_THIRD_PARTY_LICENSES).on_click(move || {
                Self::open_license(&third_party_licenses, LicenseDocument::ThirdParty);
            }),
        ))
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Left)
        .vertical_alignment(VerticalAlignment::Center);
        let footer = border(
            grid((
                documents,
                button(ui_text::CLOSE)
                    .min_width(88.0)
                    .grid_column(1)
                    .on_click(move || close.close()),
            ))
            .columns([GridLength::STAR, GridLength::Auto])
            .column_spacing(24.0),
        )
        .padding(Thickness::xy(32.0, 16.0))
        .background(ThemeRef::LayerFill)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness {
            top: 1.0,
            ..Thickness::uniform(0.0)
        })
        .grid_row(1);
        window.content(
            grid((body, footer))
                .rows([GridLength::STAR, GridLength::Auto])
                .into(),
            true,
        )
    }

    fn open_license(owner: &AuxiliaryWindow, document: LicenseDocument) {
        if let Err(error) = ThirdPartyLicensesWindow::open(owner.hwnd(), document) {
            tracing::error!(%error, "could not show license text");
        }
    }
}
