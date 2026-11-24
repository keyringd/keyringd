pub(in crate::application::platform::linux) struct Scan<T> {
    pub(in crate::application::platform::linux) found: Option<T>,
    pub(in crate::application::platform::linux) failure: Option<anyhow::Error>,
}

impl<T> Scan<T> {
    pub(in crate::application::platform::linux) fn map<U>(
        self,
        convert: impl FnOnce(T) -> U,
    ) -> Scan<U> {
        Scan {
            found: self.found.map(convert),
            failure: self.failure,
        }
    }
}
