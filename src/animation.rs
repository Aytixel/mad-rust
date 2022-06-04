use std::f64::consts::PI;
use std::time::{Duration, Instant};

pub struct AnimationCurve {
    curve: fn(f64) -> f64,
}

impl AnimationCurve {
    pub const LINEAR: AnimationCurve = AnimationCurve {
        curve: |coef: f64| coef,
    };
    pub const EASE_IN: AnimationCurve = AnimationCurve {
        curve: |coef: f64| 1.0 - ((coef * PI) / 2.0).cos(),
    };
    pub const EASE_OUT: AnimationCurve = AnimationCurve {
        curve: |coef: f64| ((coef * PI) / 2.0).sin(),
    };
    pub const EASE_IN_OUT: AnimationCurve = AnimationCurve {
        curve: |coef: f64| -((PI * coef).cos() - 1.0) / 2.0,
    };
}
pub struct Animation<T: Clone> {
    from: T,
    to: T,
    pub value: T,
    start_time: Instant,
    duration: Duration,
    running: bool,
    transform_closure: fn(&T, &T, &mut T, f64),
    animation_curve: AnimationCurve,
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
            animation_curve: AnimationCurve::LINEAR,
        }
    }

    pub fn to(&mut self, to: T, duration: Duration, animation_curve: AnimationCurve) {
        self.from = self.value.clone();
        self.to = to;
        self.start_time = Instant::now();
        self.duration = duration;
        self.running = true;
        self.animation_curve = animation_curve;
    }

    pub fn update(&mut self) -> bool {
        if self.running {
            let elapsed = self.start_time.elapsed();

            if elapsed > self.duration {
                self.value = self.to.clone();
                self.running = false;
            } else {
                let coef = (self.animation_curve.curve)(
                    elapsed.as_secs_f64() / self.duration.as_secs_f64(),
                );

                (self.transform_closure)(&self.from, &self.to, &mut self.value, coef);
            }

            true
        } else {
            false
        }
    }
}
