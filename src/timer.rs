use std::time::{Duration, SystemTime};

#[allow(dead_code)]
pub struct Timer {
    start: SystemTime,
}

#[allow(dead_code)]
impl Timer {
    /// Creates a new timer, with a start time of when it was created.
    pub fn new_timer() -> Self {
        Self {
            start: SystemTime::now(),
        }
    }

    /// Creates a new timer with a start time of the time provided.
    pub const fn from_time(time: &SystemTime) -> Self {
        Self {
            start: *time,
        }
    }

    /// Reset the timer to now.
    pub fn reset(&mut self) {
        self.start = SystemTime::now();
    }

    /// Returns the difference from now to the start time of this timer.
    pub fn get_time_difference(&self) -> Duration {
        SystemTime::now().duration_since(self.start).unwrap()
    }

    /// Resets the timer and returns the time right before the reset.
    pub fn get_and_reset_time(&mut self) -> Duration {
        let out = self.get_time_difference();
        self.reset();
        out
    }
}
