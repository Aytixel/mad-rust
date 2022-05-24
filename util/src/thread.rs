use sysinfo::{get_current_pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::collections::VecDeque;
use std::env::current_exe;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug)]
pub struct DualChannel<T: Clone> {
    tx: Arc<Mutex<VecDeque<T>>>,
    rx: Arc<Mutex<VecDeque<T>>>,
}

unsafe impl<T: Clone> Send for DualChannel<T> {}
unsafe impl<T: Clone> Sync for DualChannel<T> {}

impl<T: Clone> Clone for DualChannel<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}

impl<T: Clone> DualChannel<T> {
    pub fn new() -> (Self, Self) {
        let host = Arc::new(Mutex::new(VecDeque::new()));
        let child = Arc::new(Mutex::new(VecDeque::new()));

        (
            Self {
                tx: host.clone(),
                rx: child.clone(),
            },
            Self {
                tx: child,
                rx: host,
            },
        )
    }

    pub fn send(&self, t: T) {
        let mut buffer = match self.tx.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        buffer.push_back(t);
    }

    pub fn recv(&self) -> Option<T> {
        let mut buffer = match self.rx.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        buffer.pop_front()
    }

    pub fn lock_tx(&mut self) -> MutexGuard<VecDeque<T>> {
        match self.tx.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub fn lock_rx(&mut self) -> MutexGuard<VecDeque<T>> {
        match self.rx.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
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
