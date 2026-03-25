use std::time::Instant;
use rand::Rng;

fn extreme_matching_stress_test() {
    let mut engine = MatchingEngine::new();
    let mut rng = rand::thread_rng();

    let n_sell_orders = 100_000;
    let n_buy_orders = 100_000;

    let mut trades_total = 0;
    let mut latencies: Vec<u128> = Vec::with_capacity(n_sell_orders + n_buy_orders);

    // Submit SELL orders with random price & qty
    for id in 1..=n_sell_orders {
        let price = rng.gen_range(90..=110);      // multiple price levels
        let qty = rng.gen_range(10..=50);        // higher quantities for partial fills
        let order = Order {
            id: OrderId(id as u64),
            side: Side::Sell,
            price: Price(price),
            qty: Qty(qty),
            order_type: OrderType::Limit,
        };

        let start = Instant::now();
        let trades = engine.process(order);
        latencies.push(start.elapsed().as_micros());
        trades_total += trades.len();
    }

    // Submit BUY orders with random price & qty
    for id in (n_sell_orders + 1)..=(n_sell_orders + n_buy_orders) {
        let price = rng.gen_range(90..=110);      // multiple price levels
        let qty = rng.gen_range(10..=50);
        let order = Order {
            id: OrderId(id as u64),
            side: Side::Buy,
            price: Price(price),
            qty: Qty(qty),
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

    println!("Extreme Matching Stress Test:");
    println!("Total orders processed: {}", total_orders);
    println!("Total trades executed: {}", trades_total);
    println!("p50 latency: {} µs", p50);
    println!("p95 latency: {} µs", p95);
    println!("p99 latency: {} µs", p99);
    println!("Average latency per order: {:.2} µs", (total_elapsed as f64) / (total_orders as f64));
}

fn main() {
    extreme_matching_stress_test();
}

Extreme Matching Stress Test:
Total orders processed: 200000
Total trades executed: 127102
p50 latency: 0 µs
p95 latency: 1 µs
p99 latency: 2 µs
Average latency per order: 0.68 µs


