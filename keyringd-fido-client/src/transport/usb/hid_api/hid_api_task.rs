pub(super) type HidApiTask = Box<dyn FnOnce(&mut Option<hidapi::HidApi>) + Send>;
