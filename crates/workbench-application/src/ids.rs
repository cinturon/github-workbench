use ulid::Ulid;

use crate::ports::IdGenerator;

pub struct UlidGenerator;

impl IdGenerator for UlidGenerator {
    fn next(&self) -> Ulid {
        Ulid::new()
    }
}
