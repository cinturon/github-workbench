use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::ports::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 formatting cannot fail for utc now")
    }
}
