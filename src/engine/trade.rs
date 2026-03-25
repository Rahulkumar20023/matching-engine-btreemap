
use crate::types::{order_id::OrderId, price::Price, qty::Qty};

pub struct Trade{
    pub buy: OrderId,
    pub sell: OrderId,
    pub price: Price,
    pub qty: Qty,
}