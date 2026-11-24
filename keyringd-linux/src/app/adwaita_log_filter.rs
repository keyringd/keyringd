use gtk4::glib;

pub(super) struct AdwaitaLogFilter;

impl AdwaitaLogFilter {
    pub(super) fn install() {
        static INSTALL: std::sync::Once = std::sync::Once::new();
        INSTALL.call_once(|| {
            glib::log_set_handler(
                Some("Adwaita"),
                glib::LogLevels::LEVEL_MESSAGE,
                false,
                false,
                |domain, level, message| {
                    // SSH authentication prompts have no owning application window.
                    // Suppress only this advisory, not warnings or other notices.
                    if message != "AdwMessageDialog mapped without a transient parent. This is discouraged." {
                        glib::log_default_handler(domain, level, Some(message));
                    }
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "AdwaitaLogFilter::install" {
            case "Suppresses only the parentless adwaita notice" {
                verify "filter the parentless-dialog notice and retain other Adwaita messages" {
                    (|| {
                        const CHILD: &str = "KEYRINGD_ADWAITA_LOG_FILTER_TEST";
                        if std::env::var_os(CHILD).is_some() {
                            // GLib's writer is process-global; exercise it in an isolated child.
                            glib::log_set_writer_func(|level, fields| {
                                let field = |key| {
                                    fields
                                        .iter()
                                        .find(|field| field.key() == key)
                                        .and_then(|field| field.value_str())
                                        .unwrap_or("")
                                };
                                println!(
                                    "forwarded|{}|{level:?}|{}",
                                    field("GLIB_DOMAIN"),
                                    field("MESSAGE")
                                );
                                glib::LogWriterOutput::Handled
                            });
                            AdwaitaLogFilter::install();
                            AdwaitaLogFilter::install();
                            glib::g_message!(
                                "Adwaita",
                                "AdwMessageDialog mapped without a transient parent. This is discouraged."
                            );
                            glib::g_message!("Adwaita", "other notice");
                            glib::g_message!(
                                "Gtk",
                                "AdwMessageDialog mapped without a transient parent. This is discouraged."
                            );
                            glib::g_warning!(
                                "Adwaita",
                                "AdwMessageDialog mapped without a transient parent. This is discouraged."
                            );
                            return;
                        }
                        let output = std::process::Command::new(std::env::current_exe().unwrap())
                            .args([
                                "--exact",
                                "app::adwaita_log_filter::tests::suppresses_only_the_parentless_adwaita_notice",
                                "--nocapture",
                            ])
                            .env(CHILD, "1")
                            .env_remove("G_DEBUG")
                            .output()
                            .unwrap();
                        assert!(
                            output.status.success(),
                            "{}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        let output = String::from_utf8(output.stdout).unwrap();
                        assert!(
                            !output.contains("forwarded|Adwaita|Message|AdwMessageDialog"),
                            "{output}"
                        );
                        assert!(
                            output.contains("forwarded|Adwaita|Message|other notice"),
                            "{output}"
                        );
                        assert!(
                            output.contains("forwarded|Gtk|Message|AdwMessageDialog"),
                            "{output}"
                        );
                        assert!(
                            output.contains("forwarded|Adwaita|Warning|AdwMessageDialog"),
                            "{output}"
                        );
                    })();
                }
            }
        }
    }
}
