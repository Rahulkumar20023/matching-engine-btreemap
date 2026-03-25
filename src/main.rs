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


use std::hint::black_box;
use std::time::Instant;

// ── Shared config — IDENTICAL values used in bitmap tests ───────────
const WARM_UP:       usize = 10_000;
const MID_PRICE:     u64   = 100;     // BTreeMap uses 90–110 range naturally
const PRICE_SPREAD:  u64   = 20;      // orders generated MID..MID+SPREAD
const BASE_SEED:     u64   = 0xdeadbeefcafe1234;

// ── Same xorshift64 RNG used in bitmap repo ──────────────────────────
struct Rng { state: u64 }

impl Rng {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    fn next_order(&mut self) -> Order {
        let r    = self.next();
        let side = if r & 1 == 0 { Side::Buy } else { Side::Sell };

        let spread = (
            (self.next() % 10) +
            (self.next() % 10) +
            (self.next() % 10) +
            (self.next() % 10)
        ) as u64; // 0..+40

        let price = MID_PRICE + spread;

        let qty = match self.next() % 100 {
            0..=60  => 1  + self.next() % 10,
            61..=85 => 10 + self.next() % 90,
            _       => 100 + self.next() % 900,
        };

        Order {
            id:         OrderId(self.next() % 1_000_000),
            side,
            price:      Price(price as u64),
            qty:        Qty(qty),
            order_type: OrderType::Limit,
        }
    }
}

// ── Seed book with resting liquidity ────────────────────────────────
fn seed_engine(engine: &mut MatchingEngine, n: usize) {
    for i in 0..(n / 2) {
        // asks above mid
        let _ = engine.process(Order {
            id:         OrderId(9_000_000 + i as u64),
            side:       Side::Sell,
            price:      Price(MID_PRICE + 1 + (i as u64 % 50)),
            qty:        Qty(10 + (i as u64 % 90)),
            order_type: OrderType::Limit,
        });
        // bids at mid
        let _ = engine.process(Order {
            id:         OrderId(9_500_000 + i as u64),
            side:       Side::Buy,
            price:      Price(MID_PRICE),
            qty:        Qty(10 + (i as u64 % 90)),
            order_type: OrderType::Limit,
        });
    }
}

// ── Stats printer — nanoseconds, same format as bitmap ───────────────
fn print_stats(name: &str, latencies: &mut Vec<u64>) {
    latencies.sort_unstable();
    let n    = latencies.len();
    let mean = latencies.iter().sum::<u64>() / n as u64;
    let p50  = latencies[n * 50  / 100];
    let p90  = latencies[n * 90  / 100];
    let p99  = latencies[n * 99  / 100];
    let p999 = latencies[n * 999 / 1000];
    let max  = latencies[n - 1];

    println!("\n── {} ──", name);
    println!("  mean : {:>8}ns", mean);
    println!("  p50  : {:>8}ns", p50);
    println!("  p90  : {:>8}ns", p90);
    println!("  p99  : {:>8}ns", p99);
    println!("  p999 : {:>8}ns", p999);
    println!("  max  : {:>8}ns", max);
}

// ════════════════════════════════════════════════════════════════════
// TEST 1: Passive Add — pure insert, no matches
// ════════════════════════════════════════════════════════════════════
fn bench_passive_add() {
    let iterations = 1_000_000;
    let mut engine = MatchingEngine::new();
    let mut rng    = Rng::new(BASE_SEED);

    seed_engine(&mut engine, 2000);

    // Warmup
    for _ in 0..WARM_UP {
        let _ = black_box(engine.process(rng.next_order()));
    }

    // Fresh engine for clean measurement
    let mut engine = MatchingEngine::new();
    seed_engine(&mut engine, 2000);

    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let o     = rng.next_order();
        let start = Instant::now();
        let _ = black_box(engine.process(black_box(o)));
        latencies.push(start.elapsed().as_nanos() as u64);
    }

    print_stats("passive_add [BTreeMap]", &mut latencies);
}

// ════════════════════════════════════════════════════════════════════
// TEST 2: Mixed Workload — 60% passive, 30% aggressive, 10% cancel
// ════════════════════════════════════════════════════════════════════
fn bench_mixed_workload() {
    let iterations = 100_000;
    let mut engine = MatchingEngine::new();
    let mut rng    = Rng::new(BASE_SEED);
    let mut ctrl   = Rng::new(0xabcdef1234567890);

    seed_engine(&mut engine, 2000);

    // Warmup
    for _ in 0..1000 {
        let _ = black_box(engine.process(rng.next_order()));
    }

    let mut engine = MatchingEngine::new();
    seed_engine(&mut engine, 2000);

    let mut latencies  = Vec::with_capacity(iterations);
    let mut order_ids: Vec<u64> = Vec::with_capacity(iterations);
    let mut next_id    = 1_000_000u64;

    for _ in 0..iterations {
        let roll  = ctrl.next() % 100;
        let start = Instant::now();

        if roll < 60 {
            // 60% passive
            let mut o = rng.next_order();
            o.id      = OrderId(next_id);
            next_id  += 1;
            order_ids.push(o.id.0);
            let _ = black_box(engine.process(black_box(o)));

        } else if roll < 90 {
            // 30% aggressive — crosses spread
            let o = Order {
                id:         OrderId(next_id),
                side:       if roll % 2 == 0 { Side::Buy } else { Side::Sell },
                price:      Price(if roll % 2 == 0 { MID_PRICE + 5 } else { MID_PRICE + 1 }),
                qty:        Qty(10),
                order_type: OrderType::Limit,
            };
            next_id += 1;
            let _ = black_box(engine.process(black_box(o)));

        } else {
            // 10% cancel
            if !order_ids.is_empty() {
                let idx = (ctrl.next() as usize) % order_ids.len();
                let id  = order_ids.swap_remove(idx);
                black_box(engine.cancel(OrderId(id)));
            }
        }

        latencies.push(start.elapsed().as_nanos() as u64);
    }

    print_stats("mixed_workload [BTreeMap]", &mut latencies);
}

// ════════════════════════════════════════════════════════════════════
// TEST 3: Burst Load — 5 bursts of 100k each
// ════════════════════════════════════════════════════════════════════
fn bench_burst_load() {
    let bursts          = 5;
    let orders_per_burst = 100_000;
    let mut engine      = MatchingEngine::new();
    let mut rng         = Rng::new(BASE_SEED);

    seed_engine(&mut engine, 2000);

    let mut all_latencies: Vec<u64> = Vec::with_capacity(bursts * orders_per_burst);
    let mut next_id = 2_000_000u64;

    println!("\n── burst_load [BTreeMap] ──");

    for burst in 1..=bursts {
        let mut burst_latencies = Vec::with_capacity(orders_per_burst);

        for _ in 0..orders_per_burst {
            let mut o = rng.next_order();
            o.id      = OrderId(next_id);
            next_id  += 1;

            let start = Instant::now();
            let _ = black_box(engine.process(black_box(o)));
            let elapsed = start.elapsed().as_nanos() as u64;

            burst_latencies.push(elapsed);
            all_latencies.push(elapsed);
        }

        burst_latencies.sort_unstable();
        let n    = burst_latencies.len();
        let bp50 = burst_latencies[n * 50 / 100];
        let bp99 = burst_latencies[n * 99 / 100];
        let bmax = burst_latencies[n - 1];
        println!("  burst {:>2}: p50={:>6}ns  p99={:>6}ns  max={:>8}ns",
            burst, bp50, bp99, bmax);
    }

    print_stats("burst_load_overall [BTreeMap]", &mut all_latencies);
}

// ════════════════════════════════════════════════════════════════════
// TEST 4: Sweep Worst Case — large order sweeps 10 price levels
// ════════════════════════════════════════════════════════════════════
fn bench_sweep_worst_case() {
    let iterations    = 10_000;
    let mut latencies = Vec::with_capacity(iterations);

    for iter in 0..iterations {
        let mut engine = MatchingEngine::new();

        // Place 5 sell orders at each of 10 price levels
        for level in 0..10u64 {
            for j in 0..5u64 {
                let _ = engine.process(Order {
                    id:         OrderId(iter as u64 * 100 + level * 5 + j),
                    side:       Side::Sell,
                    price:      Price(MID_PRICE + 1 + level),
                    qty:        Qty(10),
                    order_type: OrderType::Limit,
                });
            }
        }

        // Large buy that sweeps all 10 levels
        let sweeper = Order {
            id:         OrderId(999_999_999),
            side:       Side::Buy,
            price:      Price(MID_PRICE + 10),
            qty:        Qty(500),
            order_type: OrderType::Limit,
        };

        let start = Instant::now();
        let _ = black_box(engine.process(black_box(sweeper)));
        latencies.push(start.elapsed().as_nanos() as u64);
    }

    print_stats("sweep_worst_case [BTreeMap]", &mut latencies);
}

// ════════════════════════════════════════════════════════════════════
// TEST 5: 1M Order Simulation — full stats report
// ════════════════════════════════════════════════════════════════════
fn simulate_1m_orders() {
    const TOTAL_ORDERS: usize = 1_000_000;

    let mut engine = MatchingEngine::new();
    let mut rng    = Rng::new(BASE_SEED);

    seed_engine(&mut engine, 4000);

    let mut total_matched    = 0usize;
    let mut total_passive    = 0usize;
    let mut total_cancelled  = 0usize;
    let mut buy_orders       = 0usize;
    let mut sell_orders      = 0usize;
    let mut total_qty_traded = 0u64;
    let mut latencies        = Vec::with_capacity(TOTAL_ORDERS);
    let mut bucket_latencies: Vec<Vec<u64>> = vec![Vec::new(); 10];

    let mut order_ids: Vec<u64> = Vec::new();
    let mut ctrl   = Rng::new(0xabcdef1234567890);
    let mut next_id = 5_000_000u64;

    let wall_start = Instant::now();

    for i in 0..TOTAL_ORDERS {
        let roll = ctrl.next() % 100;

        let start = Instant::now();

        if roll < 10 && !order_ids.is_empty() {
            // 10% cancel
            let idx = (ctrl.next() as usize) % order_ids.len();
            let id  = order_ids.swap_remove(idx);
            black_box(engine.cancel(OrderId(id)));
            total_cancelled += 1;
            latencies.push(start.elapsed().as_nanos() as u64);
        } else {
            let mut o = rng.next_order();
            o.id      = OrderId(next_id);

            match o.side {
                Side::Buy  => buy_orders  += 1,
                Side::Sell => sell_orders += 1,
            }

            order_ids.push(next_id);
            next_id += 1;

            let trades = black_box(engine.process(black_box(o)));
            let elapsed = start.elapsed().as_nanos() as u64;
            latencies.push(elapsed);

            if trades.is_empty() {
                total_passive += 1;
            } else {
                total_matched    += 1;
                total_qty_traded += trades.iter().map(|t| t.qty.0).sum::<u64>();
            }
        }

        bucket_latencies[i / 100_000].push(latencies[latencies.len() - 1]);
    }

    let wall_elapsed = wall_start.elapsed();

    latencies.sort_unstable();
    let n    = latencies.len();
    let mean = latencies.iter().sum::<u64>() / n as u64;
    let p50  = latencies[n * 50  / 100];
    let p90  = latencies[n * 90  / 100];
    let p99  = latencies[n * 99  / 100];
    let p999 = latencies[n * 999 / 1000];
    let max  = latencies[n - 1];

    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║     1M ORDER SIMULATION — BTreeMap Engine       ║");
    println!("╚══════════════════════════════════════════════════╝");

    println!("\n── ORDER FLOW ──────────────────────────────────────");
    println!("  Total ops        : {:>10}", TOTAL_ORDERS);
    println!("  Buy orders       : {:>10} ({:.1}%)", buy_orders,
        buy_orders  as f64 / TOTAL_ORDERS as f64 * 100.0);
    println!("  Sell orders      : {:>10} ({:.1}%)", sell_orders,
        sell_orders as f64 / TOTAL_ORDERS as f64 * 100.0);
    println!("  Matched          : {:>10} ({:.1}%)", total_matched,
        total_matched  as f64 / TOTAL_ORDERS as f64 * 100.0);
    println!("  Passive          : {:>10} ({:.1}%)", total_passive,
        total_passive  as f64 / TOTAL_ORDERS as f64 * 100.0);
    println!("  Cancelled        : {:>10} ({:.1}%)", total_cancelled,
        total_cancelled as f64 / TOTAL_ORDERS as f64 * 100.0);
    println!("  Total qty traded : {:>10}", total_qty_traded);

    println!("\n── THROUGHPUT ──────────────────────────────────────");
    println!("  Wall time        : {:>10.3}ms",
        wall_elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput       : {:>10.0} ops/sec",
        TOTAL_ORDERS as f64 / wall_elapsed.as_secs_f64());

    println!("\n── LATENCY PERCENTILES ─────────────────────────────");
    println!("  mean : {:>8}ns", mean);
    println!("  p50  : {:>8}ns", p50);
    println!("  p90  : {:>8}ns", p90);
    println!("  p99  : {:>8}ns", p99);
    println!("  p999 : {:>8}ns", p999);
    println!("  max  : {:>8}ns", max);

    println!("\n── LATENCY OVER TIME (per 100k batch) ──────────────");
    println!("  {:>10}  {:>8}  {:>8}  {:>8}", "batch", "p50(ns)", "p99(ns)", "max(ns)");
    for (i, bucket) in bucket_latencies.iter_mut().enumerate() {
        bucket.sort_unstable();
        let bn   = bucket.len();
        let bp50 = bucket[bn * 50 / 100];
        let bp99 = bucket[bn * 99 / 100];
        let bmax = bucket[bn - 1];
        println!("  {:>7}00k  {:>8}  {:>8}  {:>8}", i + 1, bp50, bp99, bmax);
    }

    println!("\n── LATENCY DISTRIBUTION ────────────────────────────");
    let bounds = [0u64, 100, 500, 1_000, 2_000, 5_000,
                  10_000, 50_000, 100_000, u64::MAX];
    let labels = ["  <100ns", "  <500ns", "   <1µs", "   <2µs",
                  "   <5µs",  "  <10µs",  "  <50µs", " <100µs", "  ≥100µs"];
    let mut prev = 0u64;
    for (label, &upper) in labels.iter().zip(bounds[1..].iter()) {
        let count = latencies.iter().filter(|&&x| x >= prev && x < upper).count();
        let pct   = count as f64 / n as f64 * 100.0;
        let bar   = "█".repeat((pct / 2.0) as usize);
        println!("  {}  {:>6.2}%  {}", label, pct, bar);
        prev = upper;
    }
    println!();
}

fn main() {
    println!("═══════════════════════════════════════════════════");
    println!("   BTreeMap Matching Engine — Benchmark Suite     ");
    println!("═══════════════════════════════════════════════════");
    bench_passive_add();
    bench_mixed_workload();
    bench_burst_load();
    bench_sweep_worst_case();
    simulate_1m_orders();
}
