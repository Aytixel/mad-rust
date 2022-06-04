use std::time::{Duration, Instant};

pub struct Animation<T: Clone> {
    from: T,
    to: T,
    pub value: T,
    start_time: Instant,
    duration: Duration,
    running: bool,
    transform_closure: fn(&T, &T, &mut T, f64),
}

impl<T: Clone> Animation<T> {
    pub fn new(value: T, transform_closure: fn(&T, &T, &mut T, f64)) -> Self {
        Self {
            from: value.clone(),
            to: value.clone(),
            value,
            start_time: Instant::now(),
            duration: Duration::default(),
            running: false,
            transform_closure,
        }
    }

    pub fn to(&mut self, to: T, duration: Duration) {
        self.from = self.value.clone();
        self.to = to;
        self.start_time = Instant::now();
        self.duration = duration;
        self.running = true;
    }

    pub fn update(&mut self) -> bool {
        if self.running {
            let elapsed = self.start_time.elapsed();
            let coef = elapsed.as_secs_f64() / self.duration.as_secs_f64();

            // add curve here

            if elapsed > self.duration {
                self.value = self.to.clone();
                self.running = false;
            } else {
                (self.transform_closure)(&self.from, &self.to, &mut self.value, coef);
            }

            true
        } else {
            false
        }
    }
}
