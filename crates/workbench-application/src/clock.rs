use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::ports::{Clock, Sleeper};

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 formatting cannot fail for utc now")
    }
}

pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&self, duration: std::time::Duration) {
        std::thread::sleep(duration);
    }
}
