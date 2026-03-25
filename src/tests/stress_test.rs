use std::time::{Instant, Duration};
use rand::Rng;

fn load_test() {
    let mut engine = MatchingEngine::new();
    let mut rng = rand::thread_rng();

    let n_orders = 50_000;
    let mut trades_total = 0;

    let start = Instant::now();

    for id in 1..=n_orders {
        // Random BUY or SELL
        let side = if rng.gen_bool(0.5) { Side::Buy } else { Side::Sell };

        // Random price between 90–110
        let price = rng.gen_range(90..=110);

        // Random quantity 1–10
        let qty = rng.gen_range(1..=10);

        let order = Order {
            id: OrderId(id),
            side,
            price: Price(price),
            qty: Qty(qty),
            order_type: OrderType::Limit,
        };

        let trades = engine.process(order);
        trades_total += trades.len();
    }

    let elapsed = start.elapsed();
    println!("Processed {} orders in {:?}", n_orders, elapsed);
    println!("Average latency per order: {:.2} µs", (elapsed.as_micros() as f64)/ (n_orders as f64));
    println!("Total trades executed: {}", trades_total);
}



fn load_test_with_latency() {
    let mut engine = MatchingEngine::new();
    let mut rng = rand::thread_rng();

    let n_orders = 50_000;
    let mut trades_total = 0;

    // Vector to store per-order latency
    let mut latencies: Vec<u128> = Vec::with_capacity(n_orders);

    for id in 1..=n_orders {
        // Random BUY or SELL
        let side = if rng.gen_bool(0.5) { Side::Buy } else { Side::Sell };

        // Random price between 90–110
        let price = rng.gen_range(90..=110);

        // Random quantity 1–10
        let qty = rng.gen_range(1..=10);

        let order = Order {
            id: OrderId(id as u64),
            side,
            price: Price(price),
            qty: Qty(qty),
            order_type: OrderType::Limit,
        };

        let start_order = Instant::now();
        let trades = engine.process(order);
        let elapsed_order = start_order.elapsed();
        latencies.push(elapsed_order.as_micros()); // store in µs

        trades_total += trades.len();
    }

    // Compute basic stats
    latencies.sort_unstable(); // sort to compute percentiles

    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];

    let total_elapsed: Duration = latencies.iter().map(|&u| Duration::from_micros(u as u64)).sum();

    println!("Processed {} orders", n_orders);
    println!("Total trades executed: {}", trades_total);
    println!("Total time (sum of all orders) ~ {:?} µs", total_elapsed.as_micros());
    println!("p50 latency: {} µs", p50);
    println!("p95 latency: {} µs", p95);
    println!("p99 latency: {} µs", p99);
    println!("Average latency per order: {:.2} µs", (total_elapsed.as_micros() as f64) / (n_orders as f64));
}

fn main() {
    load_test_with_latency();
}



