use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(in crate::app::ui::util) struct Cleanup(pub(in crate::app::ui::util) Arc<AtomicBool>);
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
