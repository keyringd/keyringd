// GTK removes an accessible object's D-Bus interface before its cache removal
// event necessarily reaches the client. Only those object-lifetime errors mean
// that a lookup candidate is gone; other protocol failures must remain errors.
pub(crate) fn is_removed_accessible(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        // zbus returns method-call errors as MethodError, but property reads
        // can wrap the same reply as FDO(UnknownMethod). Classify both forms.
        let detail = match cause.downcast_ref::<atspi::zbus::Error>() {
            Some(atspi::zbus::Error::MethodError(name, Some(detail), _))
                if name.as_str() == "org.freedesktop.DBus.Error.UnknownMethod" =>
            {
                Some(detail.as_str())
            }
            Some(atspi::zbus::Error::FDO(error)) => match error.as_ref() {
                atspi::zbus::fdo::Error::UnknownMethod(detail) => Some(detail.as_str()),
                _ => None,
            },
            _ => match cause.downcast_ref::<atspi::zbus::fdo::Error>() {
                Some(atspi::zbus::fdo::Error::UnknownMethod(detail)) => Some(detail.as_str()),
                _ => None,
            },
        };
        detail.is_some_and(|detail| {
            detail.starts_with("Object does not exist at path ")
                || (detail.starts_with("No such interface ")
                    && (detail.contains("org.a11y.atspi.Accessible")
                        || detail.contains("org.a11y.atspi.Component")
                        || (detail.contains("org.freedesktop.DBus.Properties")
                            && detail.contains("/a11y/")))
                    && detail.contains("object at path "))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::is_removed_accessible;
    use atspi::zbus::{self, fdo};

    keyringd_testkit::scenario! {
        group "is_removed_accessible" {
            case "Property errors distinguish removed objects from protocol failures" {
                verify "classify a removed accessibility object separately from an AT-SPI protocol error" {
                    for (error, removed) in [
                                (
                                    fdo::Error::UnknownMethod("Object does not exist at path “/a11y/removed”".into()),
                                    true,
                                ),
                                (
                                    fdo::Error::UnknownMethod(
                                        "No such interface “org.freedesktop.DBus.Properties” on object at path /a11y/removed"
                                            .into(),
                                    ),
                                    true,
                                ),
                                (
                                    fdo::Error::UnknownMethod("No such method GetRole".into()),
                                    false,
                                ),
                                (
                                    fdo::Error::UnknownMethod(
                                        "No such interface org.example.Other on object at path /a11y/live".into(),
                                    ),
                                    false,
                                ),
                                (
                                    fdo::Error::AccessDenied("Object does not exist at path /a11y/removed".into()),
                                    false,
                                ),
                            ] {
                                for error in [
                                    anyhow::Error::new(error.clone()),
                                    anyhow::Error::new(zbus::Error::from(error)),
                                ] {
                                    assert_eq!(
                                        is_removed_accessible(&error.context("observing setup")),
                                        removed
                                    );
                                }
                            }
                }
            }
        }
    }
}
