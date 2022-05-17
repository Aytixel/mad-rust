use sysinfo::{get_current_pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::env::current_exe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug)]
pub struct DualChannel<T: Clone> {
    tx: Arc<Mutex<Vec<T>>>,
    rx: Arc<Mutex<Vec<T>>>,
    can_send: Arc<AtomicBool>,
    can_unlock_tx: bool,
    can_receive: Arc<AtomicBool>,
    can_unlock_rx: bool,
}

unsafe impl<T: Clone> Send for DualChannel<T> {}
unsafe impl<T: Clone> Sync for DualChannel<T> {}

impl<T: Clone> Clone for DualChannel<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            can_send: self.can_send.clone(),
            can_unlock_tx: false,
            can_receive: self.can_receive.clone(),
            can_unlock_rx: false,
        }
    }
}

impl<T: Clone> DualChannel<T> {
    pub fn new() -> (Self, Self) {
        let host = Arc::new(Mutex::new(Vec::new()));
        let child = Arc::new(Mutex::new(Vec::new()));
        let host_can_lock = Arc::new(AtomicBool::new(true));
        let child_can_lock = Arc::new(AtomicBool::new(true));

        (
            Self {
                tx: host.clone(),
                rx: child.clone(),
                can_send: host_can_lock.clone(),
                can_unlock_tx: false,
                can_receive: child_can_lock.clone(),
                can_unlock_rx: false,
            },
            Self {
                tx: child,
                rx: host,
                can_send: child_can_lock,
                can_unlock_tx: false,
                can_receive: host_can_lock,
                can_unlock_rx: false,
            },
        )
    }

    pub fn send(&self, t: T) {
        let mut buffer = match self.tx.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        buffer.push(t);
    }

    pub fn recv(&self) -> Option<T> {
        if self.can_receive.load(Ordering::Relaxed) {
            let mut buffer = match self.rx.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };

            if buffer.len() == 0 {
                return None;
            }

            Some(buffer.remove(0))
        } else {
            None
        }
    }

    pub fn lock_tx(&mut self) -> Option<MutexGuard<Vec<T>>> {
        if self.can_send.swap(false, Ordering::Relaxed) {
            self.can_unlock_tx = true;

            Some(match self.tx.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            })
        } else {
            None
        }
    }

    pub fn unlock_tx(&mut self) {
        if self.can_unlock_tx {
            self.can_unlock_tx = false;
            self.can_send.store(true, Ordering::Relaxed);
        }
    }

    pub fn lock_rx(&mut self) -> Option<MutexGuard<Vec<T>>> {
        if self.can_receive.swap(false, Ordering::Relaxed) {
            self.can_unlock_rx = true;

            Some(match self.rx.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            })
        } else {
            None
        }
    }

    pub fn unlock_rx(&mut self) {
        if self.can_unlock_rx {
            self.can_unlock_rx = false;
            self.can_receive.store(true, Ordering::Relaxed);
        }
    }
}

pub fn kill_double() -> bool {
    if let Ok(path) = current_exe() {
        if let Ok(pid) = get_current_pid() {
            let mut sys = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );

            sys.refresh_processes_specifics(ProcessRefreshKind::new());

            for (process_pid, process) in sys.processes() {
                if process.exe() == path && pid != *process_pid {
                    return true;
                }
            }
        }
    }

    false
}
