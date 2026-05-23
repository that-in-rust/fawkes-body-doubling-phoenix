use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::application::traits::ProbeTimeBehavior;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemProbeTime;

impl ProbeTimeBehavior for SystemProbeTime {
    fn current_time_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn sleep_probe_interval(&self, duration: Duration) {
        thread::sleep(duration);
    }
}
