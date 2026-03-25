pub mod arena;
pub mod engine;
pub mod orderbook;
pub mod types;
// pub mod tests;
use std::collections::{BTreeMap, HashMap};

use crate::{
    arena::arena::Arena,
    orderbook::{book_node::BookNode, price_level::PriceLevel},
    types::{order_id::OrderId, price::Price, qty::Qty, side::Side},
};
use crate::orderbook::orderbook::OrderBook;
use crate::types::order::{Order,OrderType};
use crate::engine::trade::Trade;

use crate::engine::matching::MatchingEngine;




use std::time::Instant;
use rand::Rng;

fn burst_stress_test() {
    let mut engine = MatchingEngine::new();
    let mut rng = rand::thread_rng();

    let bursts = 5;
    let orders_per_burst = 100_000;

    let mut trades_total = 0;
    let mut latencies: Vec<u128> = Vec::with_capacity(bursts * orders_per_burst);

    for burst in 1..=bursts {
        println!("Starting burst {} with {} orders", burst, orders_per_burst);

        for i in 1..=orders_per_burst {
            let id = (burst - 1) * orders_per_burst + i;

            let side = if rng.gen_bool(0.5) { Side::Buy } else { Side::Sell };
            let price = rng.gen_range(90..=110);     // multiple price levels
            let qty = rng.gen_range(10..=50);       // partial fills possible

            let order = Order {
                id: OrderId(id as u64),
                side,
                price: Price(price),
                qty: Qty(qty),
                order_type: OrderType::Limit,
            };

            let start = Instant::now();
            let trades = engine.process(order);
            latencies.push(start.elapsed().as_micros());
            trades_total += trades.len();
        }
    }

    // Compute latency stats
    latencies.sort_unstable();
    let total_orders = latencies.len();
    let p50 = latencies[total_orders / 2];
    let p95 = latencies[(total_orders * 95) / 100];
    let p99 = latencies[(total_orders * 99) / 100];
    let total_elapsed: u128 = latencies.iter().sum();

    println!("Burst Stress Test Completed:");
    println!("Total orders processed: {}", total_orders);
    println!("Total trades executed: {}", trades_total);
    println!("p50 latency: {} µs", p50);
    println!("p95 latency: {} µs", p95);
    println!("p99 latency: {} µs", p99);
    println!("Average latency per order: {:.2} µs", (total_elapsed as f64) / (total_orders as f64));
}

fn main() {
    burst_stress_test();
}

Starting burst 1 with 100000 orders
Starting burst 2 with 100000 orders
Starting burst 3 with 100000 orders
Starting burst 4 with 100000 orders
Starting burst 5 with 100000 orders
Burst Stress Test Completed:
Total orders processed: 500000
Total trades executed: 393105
p50 latency: 1 µs
p95 latency: 2 µs
p99 latency: 4 µs
Average latency per order: 1.14 µs

