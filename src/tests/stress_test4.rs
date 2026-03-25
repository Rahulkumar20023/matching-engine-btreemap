use std::time::{Instant, Duration};

fn all_matching_load_test() {
    let mut engine = MatchingEngine::new();
    let mut trades_total = 0;
    let mut latencies: Vec<u128> = Vec::new();

    let n_sell_orders = 100_000;
    let n_buy_orders = 100_000;
    let price = 100; // all orders at same price to force matching

    // Submit SELL orders first
    for id in 1..=n_sell_orders {
        let order = Order {
            id: OrderId(id as u64),
            side: Side::Sell,
            price: Price(price),
            qty: Qty(1), // can adjust qty to stress more
            order_type: OrderType::Limit,
        };

        let start = Instant::now();
        let trades = engine.process(order);
        latencies.push(start.elapsed().as_micros());
        trades_total += trades.len();
    }

    // Submit BUY orders that match exactly
    for id in (n_sell_orders + 1)..=(n_sell_orders + n_buy_orders) {
        let order = Order {
            id: OrderId(id as u64),
            side: Side::Buy,
            price: Price(price),
            qty: Qty(1),
            order_type: OrderType::Limit,
        };

        let start = Instant::now();
        let trades = engine.process(order);
        latencies.push(start.elapsed().as_micros());
        trades_total += trades.len();
    }

    // Compute latency statistics
    latencies.sort_unstable();
    let total_orders = latencies.len();
    let p50 = latencies[total_orders / 2];
    let p95 = latencies[(total_orders * 95) / 100];
    let p99 = latencies[(total_orders * 99) / 100];
    let total_elapsed: u128 = latencies.iter().sum();

    println!("All-Matching Load Test:");
    println!("Total orders processed: {}", total_orders);
    println!("Total trades executed: {}", trades_total);
    println!("p50 latency: {} µs", p50);
    println!("p95 latency: {} µs", p95);
    println!("p99 latency: {} µs", p99);
    println!("Average latency per order: {:.2} µs", (total_elapsed as f64) / (total_orders as f64));
}

fn main() {
    all_matching_load_test();
}


All-Matching Load Test:
Total orders processed: 200000
Total trades executed: 100000
p50 latency: 0 µs
p95 latency: 1 µs
p99 latency: 1 µs
Average latency per order: 0.59 µs