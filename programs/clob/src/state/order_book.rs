use super::*;
use std::default::Default;

pub const BOOK_DEPTH: usize = 128;
pub const NULL: u8 = BOOK_DEPTH as u8;
pub const NUM_MARKET_MAKERS: usize = 64;

#[account(zero_copy)]
pub struct OrderBook {
    pub base: Pubkey,
    pub quote: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub buys: OrderList,
    pub sells: OrderList,
    pub market_makers: [MarketMaker; NUM_MARKET_MAKERS],
    pub pda_bump: u8,
    pub _padding: [u8; 7],
}

impl OrderBook {
    pub fn order_list(&mut self, side: Side) -> &mut OrderList {
        match side {
            Side::Buy => &mut self.buys,
            Side::Sell => &mut self.sells,
        }
    }
}

#[zero_copy]
pub struct OrderList {
    pub side: StoredSide,
    pub best_order_idx: u8,
    pub worst_order_idx: u8,
    pub _padding: [u8; 5],
    pub free_bitmap: FreeBitmap,
    pub orders: [Order; BOOK_DEPTH],
}

impl OrderList {
    pub fn iter(&self) -> OrderListIterator {
        OrderListIterator::new(self)
    }
}

pub struct OrderListIterator<'a> {
    i: u8,
    orders: &'a [Order],
}

impl<'a> OrderListIterator<'a> {
    pub fn new(order_list: &'a OrderList) -> Self {
        Self {
            i: order_list.best_order_idx,
            orders: &order_list.orders,
        }
    }
}

impl Iterator for OrderListIterator<'_> {
    type Item = (Order, u8);

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.i;

        if i == NULL || self.orders[i as usize].amount == 0 {
            None
        } else {
            let order = self.orders[i as usize];
            self.i = order.next_idx;
            Some((order, i))
        }
    }
}

impl OrderList {
    pub fn insert_order(
        &mut self,
        amount: u64,
        price: u64,
        ref_id: u32,
        market_maker_index: u8,
    ) -> Option<u8> {
        let mut order = Order {
            amount,
            price,
            ref_id,
            market_maker_index,
            next_idx: NULL,
            prev_idx: NULL,
            _padding: Default::default(),
        };

        // Iterate until finding an order with an inferior price. At that point,
        // insert this order between it and the order from the previous iteration.
        let mut prev_iteration_order: Option<(Order, u8)> = None;
        for (book_order, book_order_idx) in self.iter() {
            if self.is_price_better(order.price, book_order.price) {
                let order_idx = self.free_bitmap.get_first_free_chunk().unwrap_or_else(|| {
                    // If no space remains, remove the worst-priced order from
                    // the order book, and store the current order in its chunk.
                    let i = self.worst_order_idx;
                    self.delete_order(i);

                    i as usize
                });

                order.prev_idx = match prev_iteration_order {
                    Some((_, prev_order_idx)) => prev_order_idx,
                    None => NULL,
                };

                // This may evaluate to false in the rare event that this order
                // is the last one to place on the book, and the previous
                // `delete_order` removed `book_order`.
                order.next_idx = if self.orders[book_order_idx as usize].amount > 0 {
                    book_order_idx
                } else {
                    NULL
                };

                self.place_order(order, order_idx as u8);

                return Some(order_idx as u8);
            }

            prev_iteration_order = Some((book_order, book_order_idx));
        }

        // This order is inferior to all orders on the book. Place it on the
        // book iff there is free space.
        self.free_bitmap.get_first_free_chunk().map(|free_chunk| {
            order.prev_idx = match prev_iteration_order {
                Some((_, prev_order_idx)) => prev_order_idx,
                None => NULL,
            };
            order.next_idx = NULL;

            self.place_order(order, free_chunk as u8);

            free_chunk as u8
        })
    }

    fn place_order(&mut self, order: Order, i: u8) {
        if order.prev_idx == NULL {
            self.best_order_idx = i;
        } else {
            self.orders[order.prev_idx as usize].next_idx = i;
        }

        if order.next_idx == NULL {
            self.worst_order_idx = i;
        } else {
            self.orders[order.next_idx as usize].prev_idx = i;
        }

        self.orders[i as usize] = order;
        self.free_bitmap.mark_reserved(i);
    }

    pub fn delete_order(&mut self, i: u8) {
        // TODO credit the mm back the tokens

        let order = self.orders[i as usize];

        if i == self.best_order_idx {
            self.best_order_idx = order.next_idx;
        } else {
            self.orders[order.prev_idx as usize].next_idx = order.next_idx;
        }

        if i == self.worst_order_idx {
            self.worst_order_idx = order.prev_idx;
        } else {
            self.orders[order.next_idx as usize].prev_idx = order.prev_idx;
        }

        self.orders[i as usize] = Order::default();
        self.free_bitmap.mark_free(i);
    }

    /// Is `lhs` a better price than `rhs`?
    fn is_price_better(&self, lhs: u64, rhs: u64) -> bool {
        match self.side.into() {
            Side::Buy => lhs > rhs,
            Side::Sell => lhs < rhs,
        }
    }
}

/// To maximize cache hits and to minimize `OrderBook` size, this struct is
/// as small as possible. Many of its fields are implied rather than encoded.
/// Specifically,
/// * Whether it's a buy or sell is determined by whether it sits within `buys`
///   or `sells`.
/// * The amount of tokens that the market maker would receive if the order is
///   filled = (amount * price) / 1e9.
#[zero_copy]
#[derive(Default)]
pub struct Order {
    pub next_idx: u8,
    pub prev_idx: u8,
    pub market_maker_index: u8,
    pub _padding: [u8; 1],
    pub ref_id: u32,
    pub price: u64,
    pub amount: u64,
}

#[zero_copy]
pub struct MarketMaker {
    // 48 bytes
    pub base_balance: u64,
    pub quote_balance: u64,
    pub authority: Pubkey,
}
