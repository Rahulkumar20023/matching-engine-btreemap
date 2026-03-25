

#[derive(Debug)]
pub struct Slot<T> {
    pub value: Option<T>,
    pub next_free: Option<usize>,
}

