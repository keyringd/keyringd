use anyhow::{Context, Result};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Child, Command, Stdio};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
};

/// Owns the scenario process and all descendants assigned to its case job.
pub(super) struct CaseProcess {
    pub(super) child: Child,
    job: OwnedHandle,
}
impl CaseProcess {
    pub(super) fn spawn(command: &mut Command) -> Result<Self> {
        let job = unsafe {
            let handle = CreateJobObjectW(None, None)?;
            let owned = OwnedHandle::from_raw_handle(handle.0);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )?;
            owned
        };
        let child = command
            .stdin(Stdio::null())
            .spawn()
            .context("failed to start scenario process")?;
        let process = Self { child, job };
        unsafe {
            AssignProcessToJobObject(
                HANDLE(process.job.as_raw_handle()),
                HANDLE(process.child.as_raw_handle()),
            )?;
        }
        Ok(process)
    }

    /// Complete cleanup before the next case or the case's persisted-key cleanup starts.
    pub(super) fn stop(&mut self) -> Result<()> {
        let terminated = unsafe { TerminateJobObject(HANDLE(self.job.as_raw_handle()), 1) };
        let _ = self.child.kill();
        let waited = self.child.wait();
        terminated?;
        waited?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
            unsafe {
                QueryInformationJobObject(
                    Some(HANDLE(self.job.as_raw_handle())),
                    JobObjectBasicAccountingInformation,
                    &mut accounting as *mut _ as *mut _,
                    std::mem::size_of_val(&accounting) as u32,
                    None,
                )?;
            }
            if accounting.ActiveProcesses == 0 {
                return Ok(());
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "case job still owns {} processes after termination",
                accounting.ActiveProcesses
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
impl Drop for CaseProcess {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("case process cleanup failed: {error:#}");
        }
    }
}
