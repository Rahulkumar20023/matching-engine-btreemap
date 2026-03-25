

use crate::types::{order_id::OrderId, qty::Qty};

pub struct BookNode{
    pub order_id: OrderId,
    pub remaining: Qty,
    pub prev: Option<usize>,
    pub next: Option<usize>
}