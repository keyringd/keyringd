mod linux_atomic_file;
#[cfg(all(test, feature = "test-support"))]
mod platform_tests;

pub(crate) use linux_atomic_file::LinuxAtomicFile;
