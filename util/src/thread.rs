use sysinfo::{get_current_pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::collections::VecDeque;
use std::env::current_exe;
use std::sync::{
    Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
    WaitTimeoutResult,
};
use std::time::Duration;

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
        let mut buffer = self.tx.lock_poisoned();

        buffer.push_back(t);
    }

    pub fn recv(&self) -> Option<T> {
        let mut buffer = self.rx.lock_poisoned();

        buffer.pop_front()
    }

    pub fn lock_tx(&mut self) -> MutexGuard<VecDeque<T>> {
        self.tx.lock_poisoned()
    }

    pub fn lock_rx(&mut self) -> MutexGuard<VecDeque<T>> {
        self.rx.lock_poisoned()
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

pub trait MutexTrait<'a, T> {
    fn lock_poisoned(&self) -> MutexGuard<'_, T>;

    fn try_lock_poisoned(&self) -> Option<MutexGuard<'_, T>>;
}

impl<'a, T> MutexTrait<'_, T> for Mutex<T> {
    fn lock_poisoned(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn try_lock_poisoned(&self) -> Option<MutexGuard<'_, T>> {
        match self.try_lock() {
            Ok(guard) => Some(guard),
            Err(error) => match error {
                TryLockError::Poisoned(poisoned) => Some(poisoned.into_inner()),
                TryLockError::WouldBlock => None,
            },
        }
    }
}

pub trait RwLockTrait<'a, T> {
    fn read_poisoned(&self) -> RwLockReadGuard<'_, T>;

    fn try_read_poisoned(&self) -> Option<RwLockReadGuard<'_, T>>;

    fn write_poisoned(&self) -> RwLockWriteGuard<'_, T>;

    fn try_write_poisoned(&self) -> Option<RwLockWriteGuard<'_, T>>;
}

impl<'a, T> RwLockTrait<'_, T> for RwLock<T> {
    fn read_poisoned(&self) -> RwLockReadGuard<'_, T> {
        match self.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn try_read_poisoned(&self) -> Option<RwLockReadGuard<'_, T>> {
        match self.try_read() {
            Ok(guard) => Some(guard),
            Err(error) => match error {
                TryLockError::Poisoned(poisoned) => Some(poisoned.into_inner()),
                TryLockError::WouldBlock => None,
            },
        }
    }

    fn write_poisoned(&self) -> RwLockWriteGuard<'_, T> {
        match self.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn try_write_poisoned(&self) -> Option<RwLockWriteGuard<'_, T>> {
        match self.try_write() {
            Ok(guard) => Some(guard),
            Err(error) => match error {
                TryLockError::Poisoned(poisoned) => Some(poisoned.into_inner()),
                TryLockError::WouldBlock => None,
            },
        }
    }
}

pub trait CondvarTrait {
    fn wait_poisoned<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T>;

    fn wait_timeout_poisoned<'a, T>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult);

    fn wait_timeout_while_poisoned<'a, T, F: FnMut(&mut T) -> bool>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
        condition: F,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult);

    fn wait_while_poisoned<'a, T, F: FnMut(&mut T) -> bool>(
        &self,
        guard: MutexGuard<'a, T>,
        condition: F,
    ) -> MutexGuard<'a, T>;
}

impl CondvarTrait for Condvar {
    fn wait_poisoned<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        match self.wait(guard) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn wait_timeout_poisoned<'a, T>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult) {
        match self.wait_timeout(guard, dur) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn wait_timeout_while_poisoned<'a, T, F: FnMut(&mut T) -> bool>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
        condition: F,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult) {
        match self.wait_timeout_while(guard, dur, condition) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn wait_while_poisoned<'a, T, F: FnMut(&mut T) -> bool>(
        &self,
        guard: MutexGuard<'a, T>,
        condition: F,
    ) -> MutexGuard<'a, T> {
        match self.wait_while(guard, condition) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}
