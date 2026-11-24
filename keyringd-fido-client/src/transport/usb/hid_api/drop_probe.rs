pub(super) struct DropProbe(pub std::sync::mpsc::Sender<std::thread::ThreadId>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        let _ = self.0.send(std::thread::current().id());
    }
}
