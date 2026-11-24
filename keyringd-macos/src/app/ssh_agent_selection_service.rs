use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use keyringd_core::application::SettingStore;

use super::ssh_agent_selection::SshAgentSelection;
use crate::system::open_ssh::MacOsSshAgentConfiguration;

const SELECTION_KEY: &str = "macos.sshAgentSelection";

/// Coordinates the persisted choice and its application to the current session.
pub(crate) struct SshAgentSelectionService {
    settings: Arc<SettingStore>,
    configuration: MacOsSshAgentConfiguration,
    operation: Mutex<()>,
}

impl SshAgentSelectionService {
    pub(crate) fn new(
        settings: Arc<SettingStore>,
        configuration: MacOsSshAgentConfiguration,
    ) -> Self {
        Self {
            settings,
            configuration,
            operation: Mutex::new(()),
        }
    }

    pub(crate) fn has_managed_agent_configuration(&self) -> io::Result<bool> {
        self.configuration.has_managed_agent_configuration()
    }

    pub(crate) fn configure(&self) -> io::Result<()> {
        self.select(SshAgentSelection::Keyringd)
    }

    pub(crate) fn unconfigure(&self) -> io::Result<()> {
        self.select(SshAgentSelection::System)
    }

    fn select(&self, selection: SshAgentSelection) -> io::Result<()> {
        let _operation = self.lock()?;
        if selection == SshAgentSelection::Keyringd {
            self.configuration.validate_selection()?;
        }
        self.settings
            .set(SELECTION_KEY, &selection)
            .map_err(io::Error::other)?;
        self.apply(selection)
    }

    pub(crate) fn reconcile(&self) -> io::Result<()> {
        self.reconcile_with(|selection| self.apply(selection))
    }

    fn reconcile_with(
        &self,
        apply: impl FnOnce(SshAgentSelection) -> io::Result<()>,
    ) -> io::Result<()> {
        let _operation = self.lock()?;
        match self.settings.get(SELECTION_KEY)? {
            Some(selection) => apply(selection),
            None => Ok(()),
        }
    }

    fn apply(&self, selection: SshAgentSelection) -> io::Result<()> {
        match selection {
            SshAgentSelection::Keyringd => self.configuration.configure(),
            SshAgentSelection::System => self.configuration.unconfigure(),
        }
    }

    fn lock(&self) -> io::Result<MutexGuard<'_, ()>> {
        self.operation
            .lock()
            .map_err(|_| io::Error::other("SSH agent selection lock was poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::persistence::MacOsAtomicFile;
    use keyringd_core::persistence::FileSettingRepository;
    use std::path::Path;

    fn open(directory: &Path) -> SshAgentSelectionService {
        SshAgentSelectionService::new(
            Arc::new(
                SettingStore::new(Arc::new(FileSettingRepository::new(Arc::new(
                    MacOsAtomicFile::new(&directory.join("setting.json")),
                ))))
                .unwrap(),
            ),
            MacOsSshAgentConfiguration::new(directory.to_owned(), directory.join("agent.sock")),
        )
    }

    #[test]
    fn startup_reloads_each_saved_choice_without_touching_other_data() {
        for selection in [SshAgentSelection::Keyringd, SshAgentSelection::System] {
            let directory = tempfile::tempdir().unwrap();
            let first = open(directory.path());
            first.settings.set("another.key", &42).unwrap();
            first.settings.set(SELECTION_KEY, &selection).unwrap();
            drop(first);
            let restarted = open(directory.path());
            let applied = std::cell::Cell::new(None);
            restarted
                .reconcile_with(|choice| {
                    applied.set(Some(choice));
                    Ok(())
                })
                .unwrap();
            assert_eq!(applied.get(), Some(selection));
            assert_eq!(
                restarted.settings.get::<i32>("another.key").unwrap(),
                Some(42)
            );
        }
    }

    #[test]
    fn an_absent_choice_leaves_the_session_unchanged_and_invalid_data_is_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let service = open(directory.path());
        service
            .reconcile_with(|_| panic!("no choice was saved"))
            .unwrap();
        assert!(!directory.path().join("setting.json").exists());
        service.settings.set(SELECTION_KEY, &true).unwrap();
        let error = service
            .reconcile_with(|_| panic!("invalid settings must not be applied"))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
