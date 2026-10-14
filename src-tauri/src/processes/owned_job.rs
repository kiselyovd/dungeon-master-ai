//! Windows process-tree ownership before any backend or WebView is created.
//! Child processes inherit the job. Its non-inheritable handle intentionally
//! remains open until process termination; Windows closes it even after a crash.
//! https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects

#[cfg(not(windows))]
pub struct OwnedProcessJob;
#[cfg(not(windows))]
impl OwnedProcessJob {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self)
    }
}

#[cfg(windows)]
mod windows {
    use std::{ffi::c_void, mem::size_of, ptr};
    type Handle = *mut c_void;
    const KILL_ON_JOB_CLOSE: u32 = 0x2000;
    const EXTENDED_LIMIT_INFORMATION: i32 = 9;
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimits {
        process_time: i64,
        job_time: i64,
        flags: u32,
        minimum_working_set: usize,
        maximum_working_set: usize,
        active_processes: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimits {
        basic: BasicLimits,
        io: [u64; 6],
        process_memory: usize,
        job_memory: usize,
        peak_process_memory: usize,
        peak_job_memory: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(
            job: Handle,
            class: i32,
            info: *const c_void,
            length: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn GetCurrentProcess() -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
    }
    pub struct OwnedProcessJob {
        _handle: usize,
    }
    impl OwnedProcessJob {
        pub fn new() -> Result<Self, &'static str> {
            // SAFETY: null attributes create a private, non-inheritable handle;
            // all pointers below refer to correctly laid out initialized data.
            let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if job.is_null() {
                return Err("desktop_process_job_create_failed");
            }
            let limits = ExtendedLimits {
                basic: BasicLimits {
                    flags: KILL_ON_JOB_CLOSE,
                    ..BasicLimits::default()
                },
                ..ExtendedLimits::default()
            };
            let configured = unsafe {
                SetInformationJobObject(
                    job,
                    EXTENDED_LIMIT_INFORMATION,
                    &limits as *const _ as *const c_void,
                    size_of::<ExtendedLimits>() as u32,
                )
            } != 0;
            if !configured {
                // No process belongs to this job yet; closing cannot kill one.
                unsafe {
                    CloseHandle(job);
                }
                return Err("desktop_process_job_configure_failed");
            }
            // SAFETY: GetCurrentProcess is the documented pseudo handle for this
            // application, never the launcher or another user's process.
            if unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) } == 0 {
                unsafe {
                    CloseHandle(job);
                }
                return Err("desktop_process_job_assign_failed");
            }
            // No Drop: closing a job containing this application would kill it
            // before normal exit callbacks finish. The OS owns final closure.
            Ok(Self {
                _handle: job as usize,
            })
        }
    }
    #[cfg(all(test, target_pointer_width = "64"))]
    mod tests {
        use super::*;
        #[test]
        fn win64_job_structures_match_sdk_layout() {
            assert_eq!(size_of::<BasicLimits>(), 64);
            assert_eq!(size_of::<ExtendedLimits>(), 144);
        }
    }
}
#[cfg(windows)]
pub use windows::OwnedProcessJob;
