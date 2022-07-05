use std::thread::sleep;
use std::time::{Duration, Instant};

pub const TIMEOUT_1S: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct Timer {
    pub frame_duration: Duration,
    pub last_update: Instant,
}

impl Timer {
    pub fn new(frame_duration: Duration) -> Self {
        Self {
            frame_duration: frame_duration,
            last_update: Instant::now(),
        }
    }

    pub fn check(&mut self) -> bool {
        if self.last_update.elapsed() >= self.frame_duration {
            self.last_update = Instant::now();

            true
        } else {
            false
        }
    }

    pub fn wait(&mut self) {
        let elapsed = self.last_update.elapsed();

        if elapsed < self.frame_duration {
            sleep(self.frame_duration - elapsed);
        }

        self.last_update = Instant::now();
    }

    pub async fn wait_async(&mut self) {
        let elapsed = self.last_update.elapsed();

        if elapsed < self.frame_duration {
            tokio::time::sleep(self.frame_duration - elapsed).await;
        }

        self.last_update = Instant::now();
    }
}
