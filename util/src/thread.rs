use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

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

pub fn kill_double() {
    if let Ok(path) = current_exe() {
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );
        let mut find = false;
        sys.refresh_processes_specifics(ProcessRefreshKind::new());

        for (_, process) in sys.processes() {
            if process.exe() == path {
                if find {
                    process.kill();
                    break;
                }
                find = true;
            }
        }
    }
}
