//! Serialize plugin initialization only. The approved plugin owns instance IPC.
//! Its Windows mutex can become visible before its message window exists.
#[cfg(windows)]
mod windows {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            attributes: *const c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut c_void;
        fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
        fn ReleaseMutex(handle: *mut c_void) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    pub struct StartupGuard(usize);

    impl StartupGuard {
        pub fn acquire(identifier: &str) -> Result<Self, &'static str> {
            let name: Vec<u16> = format!("{identifier}-plugin-startup")
                .encode_utf16()
                .chain(Some(0))
                .collect();
            // No inheritable security attributes; no campaign data or credentials.
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
            if handle.is_null() {
                return Err("native_startup_guard_unavailable");
            }
            let outcome = unsafe { WaitForSingleObject(handle, 30_000) };
            if outcome == 0 || outcome == 0x80 {
                // WAIT_OBJECT_0 or WAIT_ABANDONED: this thread owns the mutex.
                return Ok(Self(handle as usize));
            }
            unsafe { CloseHandle(handle) };
            Err("native_startup_guard_timeout")
        }
    }

    impl Drop for StartupGuard {
        fn drop(&mut self) {
            // Acquired and released by the application's main thread.
            unsafe {
                ReleaseMutex(self.0 as *mut c_void);
                CloseHandle(self.0 as *mut c_void);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{sync::mpsc, time::Duration};

        #[test]
        fn same_identity_waits_until_initialization_is_ready() {
            let identity = format!("dmai-guard-test-{}", uuid::Uuid::new_v4());
            let first = StartupGuard::acquire(&identity).unwrap();
            let (ready, received) = mpsc::channel();
            let waiter = std::thread::spawn(move || {
                let _guard = StartupGuard::acquire(&identity).unwrap();
                ready.send(()).unwrap();
            });
            assert!(received.recv_timeout(Duration::from_millis(100)).is_err());
            drop(first);
            received.recv_timeout(Duration::from_secs(5)).unwrap();
            waiter.join().unwrap();
        }

        #[test]
        fn abandoned_startup_and_other_identities_remain_available() {
            let identity = format!("dmai-guard-test-{}", uuid::Uuid::new_v4());
            let abandoned_identity = identity.clone();
            std::thread::spawn(move || {
                std::mem::forget(StartupGuard::acquire(&abandoned_identity).unwrap());
            })
            .join()
            .unwrap();
            let _recovered = StartupGuard::acquire(&identity).unwrap();
            std::thread::spawn(move || {
                let _independent = StartupGuard::acquire(&format!("{identity}-guest")).unwrap();
            })
            .join()
            .unwrap();
        }
    }
}

#[cfg(windows)]
pub use windows::StartupGuard;

#[cfg(not(windows))]
pub struct StartupGuard;
#[cfg(not(windows))]
impl StartupGuard {
    pub fn acquire(_identifier: &str) -> Result<Self, &'static str> {
        Ok(Self)
    }
}
