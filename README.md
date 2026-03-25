# Matching Engine — BTreeMap Implementation

A single-symbol limit order book and matching engine built in Rust using 
`BTreeMap<Price, VecDeque<Order>>` as the core price level data structure.

This is the **baseline implementation** in the `matching-engine-rs` series.  
For the optimized variant, see → [matching-engine-bitmap](../matching-engine-bitmap)

> ⚠️ **In-Memory Only** — all order state lives in process memory.  
> There is no persistence, no WAL, no crash recovery.  
> A process restart loses all open orders and book state.

---

## Architecture

```
MatchingEngine
├── OrderBook
│   ├── BTreeMap<Price, VecDeque<Order>>   ← bid side (descending)
│   └── BTreeMap<Price, VecDeque<Order>>   ← ask side (ascending)
└── Arena                                  ← order storage
```

### How It Works

Orders are stored in a `BTreeMap` keyed by price. Each price level holds
a `VecDeque<Order>` for FIFO (price-time priority) matching.

- **Insert**: `O(log n)` — BTreeMap finds or creates the price level
- **Best bid/ask**: `O(log n)` — `.iter().next_back()` / `.iter().next()`
- **Cancel**: `O(log n)` — lookup by price, then scan the deque
- **Match**: `O(log n)` per level consumed

### Why BTreeMap?

It is the natural, idiomatic Rust choice for an ordered price index.
Correct, readable, and easy to reason about — at the cost of 
predictable but non-trivial latency due to heap allocation and 
pointer indirection on every insert.

---

## Benchmark Results

> **Environment**: Windows 11, [Intel Core i5-12450H], Rust release mode (`--release`)  
> **Measurement**: Per-operation `Instant::now()` with `std::hint::black_box`  
> **Unit**: nanoseconds (ns)

### Latency Percentiles

| Workload | mean | p50 | p90 | p99 | p999 | max |
|---|---|---|---|---|---|---|
| passive_add | 278ns | 200ns | 400ns | 900ns | 3,000ns | 15,638µs |
| mixed_workload | 253ns | 100ns | 300ns | 800ns | 3,800ns | 2,124µs |
| sweep_worst_case | 2,524ns | 2,400ns | 2,600ns | 4,300ns | 21,000ns | 155µs |
| 1M simulation | 273ns | 200ns | 400ns | 900ns | 3,300ns | 15,392µs |

### 1M Order Simulation

```
── ORDER FLOW ──────────────────────────────────────
  Total ops        :    1,000,000
  Buy orders       :      450,149  (45.0%)
  Sell orders      :      449,988  (45.0%)
  Matched          :      417,984  (41.8%)
  Passive          :      482,153  (48.2%)
  Cancelled        :       99,863  (10.0%)
  Total qty traded :   34,072,995

── THROUGHPUT ──────────────────────────────────────
  Wall time        :      317.4ms
  Throughput       :    3,151,023  ops/sec

── LATENCY PERCENTILES ─────────────────────────────
  mean :   273ns
  p50  :   200ns
  p99  :   900ns
  p999 : 3,300ns
  max  :  15.4ms  ← OS scheduler spike
```

### Burst Load (5 × 100k orders)

```
  burst  1: p50=100ns   p99=900ns    max=2,415µs
  burst  2: p50=100ns   p99=900ns    max=4,982µs
  burst  3: p50=100ns   p99=900ns    max=10,286µs
  burst  4: p50=200ns   p99=900ns    max=1,424µs
  burst  5: p50=200ns   p99=1,000ns  max=20,619µs
```

p99 creeping from 900ns → 1,000ns under sustained burst is BTreeMap 
rebalancing cost accumulating as book depth grows.

### Latency Distribution (1M orders)

```
  <100ns    0.91%
  <500ns   93.26%  ██████████████████████████████████████████████
   <1µs     4.89%  ██
   <2µs     0.72%
   <5µs     0.16%
  <10µs     0.03%
  <50µs     0.02%
```

---

## Folder Structure

```
matching-engine-btreemap/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs                  — benchmark suite entry point
    ├── arena/
    │   ├── mod.rs
    │   └── arena.rs             — order storage
    ├── engine/
    │   ├── mod.rs
    │   ├── matching.rs          — MatchingEngine: process + cancel
    │   └── trade.rs             — Trade result type
    ├── orderbook/
    │   ├── mod.rs
    │   ├── orderbook.rs         — OrderBook: bid/ask BTreeMap
    │   ├── book_node.rs         — book node definition
    │   └── price_level.rs       — price level helpers
    └── types/
        ├── mod.rs
        ├── order.rs             — Order, OrderType
        ├── order_id.rs          — OrderId newtype
        ├── price.rs             — Price newtype
        ├── qty.rs               — Qty newtype
        └── side.rs              — Side enum
```

---

## Running

```bash
# Clone
git clone https://github.com/Rahulkumar20023/matching-engine-btreemap
cd matching-engine-btreemap

# Run full benchmark suite
cargo run --release

# Run tests
cargo test
```

---

## Known Limitations of These Benchmarks

These results should be interpreted carefully. The following limitations
apply and are disclosed for full transparency:

### 1. In-Process Measurement Only
All latency numbers measure **pure in-memory matching logic**.  
Not included: network I/O, TCP stack, FIX parsing, risk checks,
persistence writes, or market data dissemination.  
A production system would add **10–50µs** on top of these numbers.

### 2. Windows Timer Resolution Floor
`Instant::now()` on Windows has ~100ns resolution.  
Any operation faster than 100ns snaps to 0ns.  
The p50 values (100–200ns) are at or near this floor — true p50
may be lower. Linux TSC gives ~1ns resolution for accurate sub-100ns
measurement.

### 3. Single-Threaded Only
The entire benchmark runs on one thread with no contention.  
Real matching engines deal with concurrent order ingestion, which
would increase p99/p999 significantly depending on queue design.

### 4. Hot Cache
1M orders over the same price range (MID..MID+40) means the working
set fits comfortably in L2/L3 cache throughout the benchmark.  
A real multi-symbol engine with a wider price range would see higher
latency due to cache misses.

### 5. No Network Jitter
The `max` values (up to 20ms) are OS scheduler preemptions, not engine
slowdowns. In production, CPU pinning and thread isolation (`taskset`,
`isolcpus`) would eliminate most of these spikes.

### 6. Synthetic Order Distribution
Orders are generated with a uniform-spread RNG (MID..MID+40 ticks).
Real order flow has heavier clustering around the mid-price and  
occasional large outlier orders. Results may differ on real market data.

### 7. Cancel Distribution
10% of operations are cancel orders, uniformly distributed across all
submitted order IDs. Real books see ~80–90% cancel rates (most orders
are cancelled before matching), which would significantly change
match rate and book depth characteristics.

---

## Why This Implementation Exists

This repo is the **first iteration** of a multi-month project to build
a production-grade matching engine from scratch in Rust.

BTreeMap is the correct starting point:
- Idiomatic Rust, easy to reason about
- Correctness is easy to verify
- Establishes the baseline to beat

The limitations it exposes directly motivated the bitmap implementation:
- `O(log n)` best bid/ask is measurably slower than `O(1)` BSR/BSF
- Heap allocation per price level creates GC-like pause patterns at p999
- BTreeMap rebalancing causes p99 to grow under sustained burst load

---

## Comparison with Bitmap Implementation

See the [global comparison README](../README.md) for the full head-to-head
benchmark table, charts, and architectural analysis.

**Summary**:

| Metric | This (BTreeMap) | Bitmap+Arena |
|---|---|---|
| Throughput | 3.15M ops/sec | 7.16M ops/sec |
| p99 (1M sim) | 900ns | 300ns |
| sweep p50 | 2,400ns | 200ns |
| Best bid/ask | O(log n) | O(1) |
| Alloc per order | Heap | Arena (zero) |

---

## References

- [Rust BTreeMap docs](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)
- [LMAX Disruptor](https://lmax-exchange.github.io/disruptor/)
- [QuantCup Winning Solution](https://gist.github.com/druska/d6ce3f2bac74db08ee9007cdf98106ef)
- [Mechanical Sympathy Blog](https://mechanical-sympathy.blogspot.com/)

---

## What's Next

This implementation is feature-complete as a baseline.  
Active development continues in [matching-engine-bitmap](../matching-engine-bitmap):

- [ ] Risk engine (position limits, fat finger checks)
- [ ] Write-ahead log (WAL) for crash recovery
- [ ] WebSocket gateway (tokio + axum)
- [ ] Multi-symbol routing
- [ ] Criterion benchmarks with HTML reports
