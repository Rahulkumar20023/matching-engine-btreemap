use std::collections::{BTreeMap, HashMap};

use crate::{
    arena::arena::Arena,
    orderbook::{book_node::BookNode, price_level::PriceLevel},
    types::{order_id::OrderId, price::Price, qty::Qty, side::Side},
};


pub struct OrderBook{
    pub bids: BTreeMap<Price, PriceLevel>,
    pub asks: BTreeMap<Price, PriceLevel>,
}

impl OrderBook{
    pub fn new()->Self{
        Self{
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }
}

