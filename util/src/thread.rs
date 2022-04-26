use sysinfo::{get_current_pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::env::current_exe;
use std::sync::mpsc::{
    channel, Iter, Receiver, RecvError, RecvTimeoutError, SendError, Sender, TryIter, TryRecvError,
};
use std::time::Duration;

#[derive(Debug)]
pub struct DualChannel<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
}

unsafe impl<T> Send for DualChannel<T> {}
unsafe impl<T> Sync for DualChannel<T> {}

impl<T> DualChannel<T> {
    pub fn new() -> (Self, Self) {
        let (tx1, rx2) = channel::<T>();
        let (tx2, rx1) = channel::<T>();

        (Self { tx: tx1, rx: rx1 }, Self { tx: tx2, rx: rx2 })
    }

    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        self.tx.send(t)
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.rx.try_recv()
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        self.rx.recv()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.rx.iter()
    }

    pub fn try_iter(&self) -> TryIter<'_, T> {
        self.rx.try_iter()
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
