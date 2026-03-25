use std::time::{Instant, Duration};
use rand::Rng;

fn bursty_load_test() {
    let mut engine = MatchingEngine::new();
    let mut rng = rand::thread_rng();

    let base_orders = 20_000;
    let bursts = 5; // number of bursts
    let burst_multiplier = 5; // how many more orders in a burst
    let mut trades_total = 0;

    // Vector to store per-order latency
    let mut latencies: Vec<u128> = Vec::new();

    // Helper closure to submit orders
    let mut submit_orders = |n: usize, start_id: u64| {
        for id in start_id..(start_id + n as u64) {
            let side = if rng.gen_bool(0.5) { Side::Buy } else { Side::Sell };
            let price = rng.gen_range(90..=110);
            let qty = rng.gen_range(1..=10);

            let order = Order {
                id: OrderId(id),
                side,
                price: Price(price),
                qty: Qty(qty),
                order_type: OrderType::Limit,
            };

            let start_order = Instant::now();
            let trades = engine.process(order);
            let elapsed_order = start_order.elapsed();
            latencies.push(elapsed_order.as_micros());
            trades_total += trades.len();
        }
    };

    let mut current_id = 1u64;

    // Submit baseline orders
    submit_orders(base_orders, current_id);
    current_id += base_orders as u64;

    // Submit burst orders
    for burst_idx in 1..=bursts {
        let burst_orders = base_orders * burst_multiplier;
        println!("Starting burst {} with {} orders", burst_idx, burst_orders);
        submit_orders(burst_orders, current_id);
        current_id += burst_orders as u64;
    }

    // Compute latency statistics
    latencies.sort_unstable();
    let total_orders = latencies.len();
    let p50 = latencies[total_orders / 2];
    let p95 = latencies[(total_orders * 95) / 100];
    let p99 = latencies[(total_orders * 99) / 100];

    let total_elapsed: u128 = latencies.iter().sum();

    println!("Total orders processed: {}", total_orders);
    println!("Total trades executed: {}", trades_total);
    println!("p50 latency: {} µs", p50);
    println!("p95 latency: {} µs", p95);
    println!("p99 latency: {} µs", p99);
    println!("Average latency per order: {:.2} µs", (total_elapsed as f64) / (total_orders as f64));
}

fn main() {
    bursty_load_test();
}

Processed 50000 orders
Total trades executed: 36159
Total time (sum of all orders) ~ 36359 µs
p50 latency: 0 µs
p95 latency: 2 µs
p99 latency: 3 µs
Average latency per order: 0.73 µs