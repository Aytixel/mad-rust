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
    connected: bool,
}

unsafe impl<T> Send for DualChannel<T> {}
unsafe impl<T> Sync for DualChannel<T> {}

impl<T> DualChannel<T> {
    pub fn new() -> (Self, Self) {
        let (tx1, rx2) = channel::<T>();
        let (tx2, rx1) = channel::<T>();

        (
            Self {
                tx: tx1,
                rx: rx1,
                connected: true,
            },
            Self {
                tx: tx2,
                rx: rx2,
                connected: true,
            },
        )
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn send(&mut self, t: T) -> Result<(), SendError<T>> {
        let result = self.tx.send(t);

        if let Err(_) = result {
            self.connected = false;
        }

        result
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        let result = self.rx.try_recv();

        if let Err(TryRecvError::Disconnected) = result {
            self.connected = false;
        }

        result
    }

    pub fn recv(&mut self) -> Result<T, RecvError> {
        let result = self.rx.recv();

        if let Err(_) = result {
            self.connected = false;
        }

        result
    }

    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let result = self.rx.recv_timeout(timeout);

        if let Err(RecvTimeoutError::Disconnected) = result {
            self.connected = false;
        }

        result
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
