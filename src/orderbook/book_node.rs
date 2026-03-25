use crate::types::{order_id::OrderId, price::Price, qty::Qty, side::Side};

pub struct BookNode{
    pub order_id: OrderId,
    pub remaining: Qty,
    pub price: Price,
    pub side: Side,
    pub prev: Option<usize>,
    pub next: Option<usize>
}