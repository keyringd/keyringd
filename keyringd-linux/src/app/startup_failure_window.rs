use gtk4::prelude::GtkWindowExt;
use libadwaita::prelude::MessageDialogExt;

pub(super) fn show(error: &anyhow::Error) -> anyhow::Result<()> {
    libadwaita::init()?;
    // A private main loop also works when application registration or startup failed.
    let main_loop = gtk4::glib::MainLoop::new(None, false);
    let dialog = libadwaita::MessageDialog::builder()
        .title("Keyringd")
        .heading("Keyringd could not start")
        .body(format!("Details:\n{error:#}"))
        .body_use_markup(false)
        .modal(true)
        .build();
    dialog.add_response("close", "Close");
    dialog.set_close_response("close");
    let loop_for_response = main_loop.clone();
    dialog.connect_response(None, move |_, _| loop_for_response.quit());
    dialog.present();
    main_loop.run();
    Ok(())
}
