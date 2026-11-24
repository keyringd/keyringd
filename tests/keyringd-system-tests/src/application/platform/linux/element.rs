use atspi::ObjectRefOwned;

#[derive(Clone, Debug)]
pub(crate) struct Element {
    object: ObjectRefOwned,
}

impl Element {
    pub(super) fn new(object: ObjectRefOwned) -> Self {
        Self { object }
    }

    pub(super) fn object(&self) -> &ObjectRefOwned {
        &self.object
    }

    pub(crate) fn same_object(&self, other: &Self) -> bool {
        self.object == other.object
    }
}
