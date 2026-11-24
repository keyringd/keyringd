use crate::app::ui::MainThreadEventReceiverExt as _;
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use block2::RcBlock;
use dispatch2::MainThreadBound;
use keyringd_core::model::{Profile, ProfileId};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSColor, NSModalResponse,
    NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::TrustEntriesEvent;
use super::TrustEntriesState;
use super::TrustEntriesWindowCommandHandler;
use super::{TrustEntriesWindowActionTarget, TrustEntryView, TrustEntryViewProps};
use crate::app::MacOsApplication;
use crate::app::ui::Layout;
use crate::app::ui::toast::ToastOverlay;
use crate::app::ui::{ComponentModel, UiEventReceiver};
use keyringd_ui_core::trust_entries_window::TrustEntryKind;

pub(crate) struct TrustEntriesWindow {
    state: Option<TrustEntriesState>,
    content: Retained<NSView>,
    events: UiEventSender<TrustEntriesEvent>,
    targets: Vec<Retained<TrustEntriesWindowActionTarget>>,
    entry_views: Vec<TrustEntryView>,
    window: Retained<NSWindow>,
    presented: Rc<Cell<bool>>,
    toast: ToastOverlay,
    mtm: MainThreadMarker,
    profiles: tokio::sync::watch::Receiver<Vec<Profile>>,
}

impl TrustEntriesWindow {
    pub(crate) fn mount(
        profile_application: Arc<MacOsApplication>,
        runtime_handle: tokio::runtime::Handle,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events_tx, events_rx) = UiEventSender::channel();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(760.0, 720.0)),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setTitle(&NSString::from_str(ui_text::TRUST_ENTRIES_WINDOW_TITLE));
        unsafe { window.setReleasedWhenClosed(false) };
        let window_content = window.contentView().expect(
            "the initialized Public Keys window should have the content view supplied by AppKit",
        );
        // Rebuild only this view; the window also owns the persistent toast overlay.
        let content = NSView::new(mtm);
        window_content.addSubview(&content);
        Layout::pin(&content, &window_content, 0.0, 0.0, 0.0, 0.0);
        let toast = ToastOverlay::new(&window_content, mtm);
        let snapshot = profile_application.subscribe_profile_changes();
        let (_, changes) = UiEventReceiver::from_tokio_watch(&runtime_handle, snapshot.clone());
        let component = Arc::new(MainThreadBound::new(
            RefCell::new(Self {
                state: None,
                content,
                events: events_tx,
                targets: Vec::new(),
                entry_views: Vec::new(),
                window,
                presented: Rc::new(Cell::new(false)),
                toast,
                mtm,
                profiles: snapshot,
            }),
            mtm,
        ));
        let weak = Arc::downgrade(&component);
        events_rx.spawn_on_main(&runtime_handle, move |event, mtm| {
            if let Some(component) = weak.upgrade() {
                component.get(mtm).borrow_mut().dispatch(event);
            }
        });
        let weak = Arc::downgrade(&component);
        changes.spawn_on_main(&runtime_handle, move |_, mtm| {
            if let Some(component) = weak.upgrade() {
                component.get(mtm).borrow_mut().profiles_changed();
            }
        });
        component
    }

    pub(crate) fn present(&mut self, profile_id: ProfileId, parent: &NSWindow) {
        self.present_with_notice(profile_id, parent, None);
    }

    pub(crate) fn present_with_notice(
        &mut self,
        profile_id: ProfileId,
        parent: &NSWindow,
        notice: Option<String>,
    ) {
        if self.presented.get() || parent.attachedSheet().is_some() {
            return;
        }
        let entries = self
            .profiles
            .borrow()
            .iter()
            .find(|profile| profile.id() == &profile_id)
            .map(TrustEntriesState::from_profile);
        let Some(entries) = entries else { return };
        let state = entries.with_notice(notice);
        self.render(&state);
        self.state = Some(state);
        self.presented.set(true);
        let presented = Rc::clone(&self.presented);
        let window = self.window.clone();
        let completion = RcBlock::new(move |_: NSModalResponse| {
            window.orderOut(None);
            presented.set(false);
        });
        parent.beginSheet_completionHandler(&self.window, Some(&completion));
    }

    fn profiles_changed(&mut self) {
        if !self.window.isVisible() {
            return;
        }
        let Some(state) = &self.state else {
            return;
        };
        let id = state.profile_id().clone();
        let event = self
            .profiles
            .borrow()
            .iter()
            .find(|profile| profile.id() == &id)
            .map(|profile| {
                TrustEntriesEvent::EntriesChanged(TrustEntriesState::from_profile(profile))
            })
            .unwrap_or(TrustEntriesEvent::ProfileRemoved);
        self.dispatch(event);
    }

    fn dispatch(&mut self, event: TrustEntriesEvent) {
        let Some(mut state) = self.state.take() else {
            return;
        };
        let command = state.update(event);
        self.render(&state);
        self.state = Some(state);
        if let Some(command) = command {
            TrustEntriesWindowCommandHandler::handle(command, &self.window, &self.toast, self.mtm);
        }
    }

    fn render(&mut self, state: &TrustEntriesState) {
        for view in self.content.subviews() {
            view.removeFromSuperview();
        }
        self.targets.clear();
        self.entry_views.clear();
        let mtm = self.mtm;
        let heading = Layout::label(&state.heading(), 22.0, true, mtm);
        heading.setAccessibilityIdentifier(Some(&NSString::from_str("trust-entries.heading")));
        let intro = Layout::label(ui_text::TRUST_ENTRIES_DESCRIPTION, 13.0, false, mtm);
        let ca_title = Layout::label(ui_text::TRUST_ENTRIES_CERTIFICATE_SERVERS, 16.0, true, mtm);
        let ca_description = Layout::label(
            ui_text::TRUST_ENTRIES_CERTIFICATE_DESCRIPTION,
            12.0,
            false,
            mtm,
        );
        let ca_entry = self.entry(TrustEntryViewProps {
            kind: TrustEntryKind::CertificateAuthority,
            description: ui_text::TRUST_ENTRIES_CA_DESCRIPTION,
            value: state.value(TrustEntryKind::CertificateAuthority),
        });
        let ca_group = Layout::group(&[&ca_title, &ca_description, ca_entry.view()], false, mtm);
        let direct_title = Layout::label(ui_text::TRUST_ENTRIES_DIRECT_SERVERS, 16.0, true, mtm);
        let direct_description =
            Layout::label(ui_text::TRUST_ENTRIES_DIRECT_DESCRIPTION, 12.0, false, mtm);
        let device_entry = self.entry(TrustEntryViewProps {
            kind: TrustEntryKind::DeviceKey,
            description: ui_text::TRUST_ENTRIES_SECURE_ENCLAVE_KEY_DESCRIPTION,
            value: state.value(TrustEntryKind::DeviceKey),
        });
        let passkey_entry = self.entry(TrustEntryViewProps {
            kind: TrustEntryKind::Passkey,
            description: ui_text::TRUST_ENTRIES_PASSKEY_DESCRIPTION,
            value: state.value(TrustEntryKind::Passkey),
        });
        let copy_both = self.button(
            TrustEntryKind::DirectKeys.copy_label(),
            "trust-entries.copy-direct-keys",
            TrustEntriesEvent::CopyRequested(TrustEntryKind::DirectKeys),
        );
        let direct_actions = Layout::actions(&[&copy_both], mtm);
        let direct_group = Layout::group(
            &[
                &direct_title,
                &direct_description,
                device_entry.view(),
                passkey_entry.view(),
                &direct_actions,
            ],
            false,
            mtm,
        );
        let close = self.button(
            ui_text::CLOSE,
            "trust-entries.close",
            TrustEntriesEvent::CloseRequested,
        );
        close.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        let actions = Layout::actions(&[&close], mtm);
        let banner = state.registration_notice().map(|message| {
            let title = Layout::label(
                &format!("⚠ {}", ui_text::TRUST_ENTRIES_REGISTRATION_UPDATE),
                13.0,
                true,
                mtm,
            );
            let notice = Layout::label(message, 13.0, false, mtm);
            notice.setTextColor(Some(&NSColor::labelColor()));
            notice.setAccessibilityIdentifier(Some(&NSString::from_str(
                "public-keys.registration-update",
            )));
            Layout::group(&[&title, &notice], true, mtm)
        });
        let mut views: Vec<&NSView> = vec![&heading, &intro];
        if let Some(banner) = &banner {
            views.push(banner);
        }
        views.extend_from_slice(&[&ca_group, &direct_group, &actions]);
        let stack = Layout::stack(
            &views,
            NSUserInterfaceLayoutOrientation::Vertical,
            20.0,
            mtm,
        );
        self.content.addSubview(&stack);
        Layout::pin(&stack, &self.content, 28.0, 28.0, 20.0, 28.0);
        Layout::width(&stack, 704.0);
        self.entry_views
            .extend([ca_entry, device_entry, passkey_entry]);
        Layout::fit_window(&self.content);
    }
    fn entry(&self, props: TrustEntryViewProps) -> TrustEntryView {
        let identifier = match props.kind {
            TrustEntryKind::CertificateAuthority => "trust-entries.copy-ca",
            TrustEntryKind::DeviceKey => "trust-entries.copy-device-key",
            TrustEntryKind::Passkey => "trust-entries.copy-passkey",
            TrustEntryKind::DirectKeys => "trust-entries.copy-direct-keys",
        };
        TrustEntryView::render(props, self.events.clone(), identifier, self.mtm)
    }
    fn button(&mut self, title: &str, id: &str, event: TrustEntriesEvent) -> Retained<NSButton> {
        let target = TrustEntriesWindowActionTarget::new(self.mtm, self.events.clone(), event);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(title),
                Some(&target),
                Some(sel!(performAction:)),
                self.mtm,
            )
        };
        button.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
        self.targets.push(target);
        button
    }
}
