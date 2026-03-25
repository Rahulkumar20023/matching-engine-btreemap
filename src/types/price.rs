

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Price(pub u64);

impl Ord for Price{
    fn cmp(&self, other:&Self)->Ordering{
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Price{
    fn partial_cmp(&self, other:&Self)->Option<Ordering>{
        Some(self.cmp(other))
    }
}