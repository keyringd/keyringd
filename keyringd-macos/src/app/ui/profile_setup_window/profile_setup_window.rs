use crate::app::ui::MainThreadEventReceiverExt as _;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use dispatch2::MainThreadBound;
use keyringd_core::application::ActionId;
use keyringd_core::application::ProfileSetupFailure;
use keyringd_core::model::Profile;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use std::{cell::RefCell, sync::Arc};

use super::ProfileSetupWindowCommandHandler;
use crate::app::MacOsApplication;
use crate::app::ui::profile_setup_wizard::ProfileSetupWizardView;
use crate::app::ui::{ComponentRuntime, PASSKEY_DIALOG_WIDTH};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profile_setup_window::{
    ProfileSetupWindowCommand, ProfileSetupWindowEvent, ProfileSetupWindowState,
};

pub(crate) struct ProfileSetupWindow {
    window: Retained<NSWindow>,
    wizard: ProfileSetupWizardView,
    runtime: ComponentRuntime<ProfileSetupWindowState>,
    flow: Option<ActionId>,
    trust_entries: Arc<MainThreadBound<RefCell<TrustEntriesWindow>>>,
    mtm: MainThreadMarker,
    key_cancellation: Option<tokio_util::sync::DropGuard>,
}

impl ProfileSetupWindow {
    pub(crate) fn new(
        runtime: tokio::runtime::Handle,
        application: Arc<MacOsApplication>,
        interaction: Arc<KeyOperationCoordinator>,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events, receiver) = UiEventSender::channel();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(PASSKEY_DIALOG_WIDTH, 480.0),
                ),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setTitle(&NSString::from_str(ui_text::PROFILE_SETUP_WINDOW_TITLE));
        unsafe { window.setReleasedWhenClosed(false) };
        let content = window.contentView().expect(
            "the initialized Profile Setup sheet should have the content view supplied by AppKit",
        );
        let wizard = ProfileSetupWizardView::new(
            &content,
            UiEventTarget::from_sender(events.clone()).map(ProfileSetupWindowEvent::from),
            mtm,
        );
        let component = Arc::new(MainThreadBound::new(
            RefCell::new(Self {
                window,
                wizard,
                runtime: ComponentRuntime::new(ProfileSetupWindowState::default()),
                flow: None,
                trust_entries: TrustEntriesWindow::mount(
                    Arc::clone(&application),
                    runtime.clone(),
                    mtm,
                ),
                mtm,
                key_cancellation: None,
            }),
            mtm,
        ));
        component.get(mtm).borrow().render();
        let weak = Arc::downgrade(&component);
        let handler = ProfileSetupWindowCommandHandler::new(
            application,
            runtime.clone(),
            interaction.clone(),
        );
        receiver.spawn_on_main(&runtime, move |event, mtm| {
            let Some(setup) = weak.upgrade() else {
                return;
            };
            let command = setup.get(mtm).borrow().dispatch(event);
            if let Some(command) = command {
                handler.handle(command, mtm, setup);
            }
        });
        component
    }

    pub(super) fn bind_flow(&mut self, id: ActionId, cancellation: tokio_util::sync::DropGuard) {
        assert!(
            self.flow.replace(id).is_none(),
            "profile setup is already running"
        );
        self.key_cancellation = Some(cancellation);
    }
    pub(crate) fn dispatch(
        &self,
        event: impl Into<ProfileSetupWindowEvent>,
    ) -> Option<ProfileSetupWindowCommand> {
        let span = self.flow.map(ActionId::span);
        let _entered = span.as_ref().map(tracing::Span::enter);
        self.runtime
            .dispatch_and_render(event.into(), |state| self.render_state(state))
    }

    pub(crate) fn show_for(&self, parent: &NSWindow) {
        if self.flow.is_some() || self.window.sheetParent().is_some() {
            self.window.makeKeyAndOrderFront(None);
            return;
        }
        self.runtime
            .reset(ProfileSetupWindowState::default(), |state| {
                self.render_state(state)
            });
        parent.beginSheet_completionHandler(&self.window, None);
        self.wizard.reset_input();
    }

    pub(super) fn show_trust_entries(&self, id: keyringd_core::model::ProfileId) {
        self.trust_entries
            .get(self.mtm)
            .borrow_mut()
            .present(id, &self.window);
    }

    pub(crate) fn hide(&self) {
        if let Some(parent) = self.window.sheetParent() {
            parent.endSheet(&self.window);
        }
        self.window.orderOut(None);
    }

    pub(crate) fn finish_flow(
        &mut self,
        id: ActionId,
        result: Arc<Result<Profile, ProfileSetupFailure>>,
    ) -> Option<ProfileSetupWindowCommand> {
        let span = id.span();
        let _entered = span.enter();
        if self.flow != Some(id) {
            return None;
        }
        self.flow = None;
        self.key_cancellation = None;
        self.dispatch(ProfileSetupWindowEvent::Finished(result))
    }

    pub(crate) fn flow_id(&self) -> Option<ActionId> {
        self.flow
    }

    fn render(&self) {
        self.render_state(&self.runtime.state());
    }

    fn render_state(&self, state: &ProfileSetupWindowState) {
        self.wizard.render(state);
        let content = self.window.contentView().expect(
            "the initialized Profile Setup sheet should have the content view supplied by AppKit",
        );
        content.layoutSubtreeIfNeeded();
        let required = content.fittingSize();
        let current = content.bounds().size;
        // Grow the sheet for a page that needs more room, but never give the
        // room back: a later page finds the sheet already tall enough and the
        // height stops moving once the flow has shown its tallest page.
        self.window.setContentSize(NSSize::new(
            PASSKEY_DIALOG_WIDTH,
            current.height.max(required.height),
        ));
    }
}
