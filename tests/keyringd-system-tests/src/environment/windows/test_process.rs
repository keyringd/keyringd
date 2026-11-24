use std::io;
use std::os::windows::process::ExitStatusExt;
use std::process::{Child, ExitStatus};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

pub(crate) enum TestProcess {
    Child(Child),
    Activated { handle: HANDLE, pid: u32 },
}

// The owned process handle can be used by the fixture watchdog thread.
unsafe impl Send for TestProcess {}

impl TestProcess {
    pub(crate) fn activated(pid: u32) -> io::Result<Self> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        }?;
        Ok(Self::Activated { handle, pid })
    }

    pub(crate) fn id(&self) -> u32 {
        match self {
            Self::Child(child) => child.id(),
            Self::Activated { pid, .. } => *pid,
        }
    }

    pub(crate) fn kill(&mut self) -> io::Result<()> {
        match self {
            Self::Child(child) => child.kill(),
            Self::Activated { handle, .. } => {
                unsafe { TerminateProcess(*handle, 1) }.map_err(Into::into)
            }
        }
    }

    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        match self {
            Self::Child(child) => child.try_wait(),
            Self::Activated { handle, .. } => {
                if unsafe { WaitForSingleObject(*handle, 0) } != WAIT_OBJECT_0 {
                    return Ok(None);
                }
                let mut code = 0;
                unsafe { GetExitCodeProcess(*handle, &mut code) }?;
                Ok(Some(ExitStatus::from_raw(code)))
            }
        }
    }

    pub(crate) fn wait(&mut self) -> io::Result<ExitStatus> {
        match self {
            Self::Child(child) => child.wait(),
            Self::Activated { handle, .. } => {
                unsafe { WaitForSingleObject(*handle, 15_000) };
                self.try_wait()?.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::TimedOut, "activated fixture did not exit")
                })
            }
        }
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        if let Self::Activated { handle, .. } = self {
            unsafe {
                let _ = CloseHandle(*handle);
            }
        }
    }
}
