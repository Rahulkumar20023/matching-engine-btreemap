use std::time::{Instant, Duration};
use rand::Rng;

fn extreme_load_test(n_orders: usize) {
    let mut engine = MatchingEngine::new();
    let mut rng = rand::thread_rng();

    let mut trades_total = 0;
    let mut latencies: Vec<u128> = Vec::with_capacity(n_orders);

    for id in 1..=n_orders {
        let side = if rng.gen_bool(0.5) { Side::Buy } else { Side::Sell };
        let price = rng.gen_range(90..=110);
        let qty = rng.gen_range(1..=100);

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

    latencies.sort_unstable();
    let total_orders = latencies.len();
    let p50 = latencies[total_orders / 2];
    let p95 = latencies[(total_orders * 95) / 100];
    let p99 = latencies[(total_orders * 99) / 100];
    let total_elapsed: u128 = latencies.iter().sum();

    println!("Extreme Load Test: {} orders", total_orders);
    println!("Total trades executed: {}", trades_total);
    println!("p50: {} µs, p95: {} µs, p99: {} µs", p50, p95, p99);
    println!("Average latency per order: {:.2} µs", (total_elapsed as f64) / (total_orders as f64));
}

fn main() {
    extreme_load_test(1_000_000); // start with 1 million orders
}
Starting burst 1 with 100000 orders
Starting burst 2 with 100000 orders
Starting burst 3 with 100000 orders
Starting burst 4 with 100000 orders
Starting burst 5 with 100000 orders
Total orders processed: 520000
Total trades executed: 377081
p50 latency: 0 µs
p95 latency: 2 µs
p99 latency: 3 µs
Average latency per order: 0.92 µs