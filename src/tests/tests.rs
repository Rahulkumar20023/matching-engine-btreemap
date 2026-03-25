pub mod arena;
pub mod engine;
pub mod orderbook;
pub mod types;

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


#[cfg(test)]
mod tests {
    use crate::engine::matching::MatchingEngine;
    use crate::types::{
        order::Order,
        order::OrderType,
        order_id::OrderId,
        price::Price,
        qty::Qty,
        side::Side,
    };

    // ----------------------------
    // Helpers
    // ----------------------------

    fn limit(
        id: u64,
        side: Side,
        price: u64,
        qty: u64,
    ) -> Order {
        Order {
            id: OrderId(id),
            price: Price(price),
            qty: Qty(qty),
            side,
            order_type: OrderType::Limit,
        }
    }

    fn market(
        id: u64,
        side: Side,
        qty: u64,
    ) -> Order {
        Order {
            id: OrderId(id),
            price: Price(0), // unused for market
            qty: Qty(qty),
            side,
            order_type: OrderType::Market,
        }
    }

    // ----------------------------
    // Test 1: Limit order rests
    // ----------------------------
    #[test]
    fn limit_order_rests_when_no_cross() {
        let mut engine = MatchingEngine::new();

        let trades = engine.process(
            limit(1, Side::Buy, 100, 10)
        );

        assert!(trades.is_empty());
        assert_eq!(engine.book.bids.len(), 1);
        assert!(engine.book.asks.is_empty());
    }

    // ----------------------------
    // Test 2: FIFO at same price
    // ----------------------------
    #[test]
    fn fifo_same_price() {
        let mut engine = MatchingEngine::new();

        engine.process(limit(1, Side::Sell, 100, 10));
        engine.process(limit(2, Side::Sell, 100, 10));

        let trades = engine.process(
            limit(3, Side::Buy, 100, 15)
        );

        assert_eq!(trades.len(), 2);

        // FIFO: order 1 must fill first
        assert_eq!(trades[0].sell, OrderId(1));
        assert_eq!(trades[0].qty, Qty(10));

        // Then order 2
        assert_eq!(trades[1].sell, OrderId(2));
        assert_eq!(trades[1].qty, Qty(5));
    }

    // ----------------------------
    // Test 3: Partial fill remainder
    // ----------------------------
    #[test]
    fn partial_fill_leaves_remainder() {
        let mut engine = MatchingEngine::new();

        engine.process(limit(1, Side::Sell, 100, 10));

        let trades = engine.process(
            limit(2, Side::Buy, 100, 4)
        );

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].qty, Qty(4));

        let level = engine.book.asks.get(&Price(100)).unwrap();
        let node = engine.arena.get(level.head.unwrap()).unwrap();

        assert_eq!(node.remaining, Qty(6));
    }

    // ----------------------------
    // Test 4: Market order never rests
    // ----------------------------
    #[test]
    fn market_order_does_not_rest() {
        let mut engine = MatchingEngine::new();

        let trades = engine.process(
            market(1, Side::Buy, 10)
        );

        assert!(trades.is_empty());
        assert!(engine.book.bids.is_empty());
        assert!(engine.book.asks.is_empty());
    }

    // ----------------------------
    // Test 5: Cancel removes order
    // ----------------------------
    #[test]
    fn cancel_removes_order() {
        let mut engine = MatchingEngine::new();

        engine.process(limit(1, Side::Buy, 100, 10));

        let ok = engine.cancel(OrderId(1));
        assert!(ok);

        assert!(engine.book.bids.is_empty());
        assert!(engine.order_index.is_empty());
    }

    // ----------------------------
    // Test 6: Cancel after fill fails
    // ----------------------------
    #[test]
    fn cancel_filled_order_fails() {
        let mut engine = MatchingEngine::new();

        engine.process(limit(1, Side::Sell, 100, 10));
        engine.process(limit(2, Side::Buy, 100, 10));

        let ok = engine.cancel(OrderId(1));
        assert!(!ok);
    }

    #[test]
    fn invariant_no_empty_price_levels() {
        let mut engine = MatchingEngine::new();

        // Create some activity
        engine.process(limit(1, Side::Sell, 100, 10));
        engine.process(limit(2, Side::Sell, 101, 10));
        engine.process(limit(3, Side::Buy, 100, 5)); // partial fill

        // Check bids
        for (_, level) in &engine.book.bids {
            assert!(
                !level.is_empty(),
                "Found empty bid price level"
            );
        }

        // Check asks
        for (_, level) in &engine.book.asks {
            assert!(
                !level.is_empty(),
                "Found empty ask price level"
            );
        }
    }

    #[test]
fn invariant_quantity_conservation_sell_side_many_orders() {
    let mut engine = MatchingEngine::new();

    let mut submitted_sell = 0u64;
    let mut traded = 0u64;

    // Sell orders
    submitted_sell += 50;
    engine.process(limit(1, Side::Sell, 100, 50));

    submitted_sell += 30;
    engine.process(limit(2, Side::Sell, 101, 30));

    submitted_sell += 20;
    engine.process(limit(3, Side::Sell, 102, 20));

    // Buy orders
    let trades1 = engine.process(limit(4, Side::Buy, 102, 40));
    let trades2 = engine.process(limit(5, Side::Buy, 101, 30));

    for t in trades1.into_iter().chain(trades2) {
        traded += t.qty.0;
    }

    // Remaining sell quantity
    let mut resting_sell = 0u64;
    for (_, level) in &engine.book.asks {
        let mut cur = level.head;
        while let Some(idx) = cur {
            let node = engine.arena.get(idx).unwrap();
            resting_sell += node.remaining.0;
            cur = node.next;
        }
    }

    assert_eq!(
        submitted_sell,
        traded + resting_sell,
        "Sell-side quantity conservation violated"
    );
}


#[test]
#[test]
fn invariant_cancel_aware_quantity_conservation() {
    let mut engine = MatchingEngine::new();

    let mut submitted_sell = 0u64;
    let mut traded = 0u64;
    let mut cancelled = 0u64;

    // Submit sell orders
    submitted_sell += 50;
    engine.process(limit(1, Side::Sell, 100, 50));

    submitted_sell += 30;
    engine.process(limit(2, Side::Sell, 101, 30));

    // Buy partially matches
    let trades = engine.process(limit(3, Side::Buy, 101, 40));
    for t in trades {
        traded += t.qty.0;
    }

    // Compute remaining quantity before cancel
    if let Some(idx) = engine.order_index.get(&OrderId(2)) {
        let node = engine.arena.get(*idx).unwrap();
        cancelled += node.remaining.0;
    }

    // Cancel the order
    let cancelled_ok = engine.cancel(OrderId(2));
    assert!(cancelled_ok);

    // Remaining sell quantity in book
    let mut resting_sell = 0u64;
    for (_, level) in &engine.book.asks {
        let mut cur = level.head;
        while let Some(idx) = cur {
            let node = engine.arena.get(idx).unwrap();
            resting_sell += node.remaining.0;
            cur = node.next;
        }
    }

    assert_eq!(
        submitted_sell,
        traded + resting_sell + cancelled,
        "Cancel-aware quantity conservation violated"
    );
}




}