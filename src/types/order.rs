use crate::types::{order_id::OrderId, price::Price, side::Side,qty::Qty};

#[derive(Debug, Clone)]
pub enum OrderType{
    Limit,
    Market,
}

#[derive(Debug, Clone)]
pub struct Order{
    pub id: OrderId,
    pub price: Price,
    pub qty: Qty,
    pub side: Side,
    pub order_type: OrderType,
}