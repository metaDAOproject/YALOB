use super::*;
use std::default::Default;

pub const BOOK_DEPTH: usize = 128;
pub const NULL: u8 = BOOK_DEPTH as u8; //
pub const NUM_MARKET_MAKERS: usize = 64;

#[account(zero_copy)]
pub struct OrderBook {
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub buys: OrderList,
    pub sells: OrderList,
    pub market_makers: [MarketMaker; NUM_MARKET_MAKERS],
}

#[zero_copy]
pub struct OrderList {
    pub side: StoredSide,
    pub best_order_index: u8,
    pub worst_order_index: u8,
    pub _padding: [u8; 5],
    pub free_bitmap: FreeBitmap,
    pub orders: [Order; BOOK_DEPTH],
}

pub struct OrderListIterator<'a> {
    i: u8,
    orders: &'a [Order],
}

impl<'a> OrderListIterator<'a> {
    fn new(order_list: &'a OrderList) -> Self {
        Self {
            i: order_list.best_order_index,
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
            self.i = order.next_index;
            Some((order, i))
        }
    }
}

impl OrderList {
    pub fn insert_order(&mut self, mut order: Order) -> Option<u8> {
        let mut iter = OrderListIterator::new(self);

        let mut prev_order: Option<(Order, u8)> = None;
        while let Some((mut book_order, i)) = iter.next() {
            if self.is_price_better(order, book_order) {
                // if all chunks are taken, delete the worst order
                if self.free_bitmap.are_all_chunks_taken() {
                    // TODO credit the mm back their tokens
                    self.orders[self.worst_order_index as usize] = Order::default();
                    self.free_bitmap.mark_free(self.worst_order_index)
                }

                // this must not be `None` since we just deleted an order if all
                // chunks were taken
                let free_chunk = self.free_bitmap.get_first_free_chunk().unwrap();

                if let Some((mut prev_order, i)) = prev_order {
                    prev_order.next_index = free_chunk as u8;
                    self.orders[i as usize] = prev_order;
                    order.prev_index = i;
                } else {
                    self.best_order_index = free_chunk as u8;
                    order.prev_index = NULL;
                }

                book_order.prev_index = free_chunk as u8;
                self.orders[i as usize] = book_order;

                order.next_index = i;

                self.orders[free_chunk] = order;

                return Some(free_chunk as u8);
            }

            prev_order = Some((book_order, i));
        }

        // the order isn't better than any on the book. If there's a free
        // chunk, place it there. 
        self.free_bitmap.get_first_free_chunk().map(|free_chunk| {
            order.prev_index = match prev_order {
                Some((mut prev_order, i)) => {
                    prev_order.next_index = free_chunk as u8;
                    self.orders[i as usize] = prev_order;
                    i
                },
                // a `None` condition only arises when the order is simultaneously
                // better than no orders on the book and better than all orders on
                // the book, which can only happen when there are no orders on the
                // book
                None => NULL,
            };
            order.next_index = NULL;

            self.orders[free_chunk] = order;
            self.free_bitmap.mark_reserved(free_chunk as u8);
            self.worst_order_index = free_chunk as u8;

            free_chunk.try_into().unwrap()
        })
    }

    fn _delete_order(&mut self, i: u8) {
        // TODO credit the mm back the tokens
        let order_to_delete = self.orders[i as usize];

        if order_to_delete.prev_index != NULL {

        }

        self.orders[i as usize] = Order::default();
    }

    /// Does `lhs` give a better price than `rhs`?
    fn is_price_better(&self, lhs: Order, rhs: Order) -> bool {
        match self.side.into() {
            Side::Buy => lhs.price > rhs.price,
            Side::Sell => lhs.price < rhs.price,
        }
    }
}

// impl OrderList {
//     fn insert_order(&mut self, order: Order) -> Option<u8> {
//         let mut i = self.best_order_index as usize;

//         // find the first order in the list that this order beats on price
//         let first_worse_order = self.get_first_worse_order(order);

//         match first_worse_order {
//             None => {
//                 if self.size as usize == BOOK_DEPTH {
//                     None
//                 } else {
//                     let bump_index = self.bump_index;
//                     if (bump_index as usize) < BOOK_DEPTH {
//                         order.prev_index = self.worst_order_index;
//                         self.worst_order_index = bump_index;
//                         order.next_index = NULL;
//                         self.orders[bump_index as usize] = order;

//                         self.bump_index += 1;

//                         Some(bump_index)
//                     } else {
//                         None
//                     }
//                 }
//             },
//             Some(first_worst_order) => {
//                 let slightly_better_order = self.orders[first_worse_order.prev_index as usize];





//             }
//         }
//     }

//     /// Iterate through the order list, returning the first order that has a
//     /// worse price than `order`. Returns `None` if this order has a worse
    /// price than every order in the list.
//     fn get_first_worse_order(&self, order: Order) -> Option<Order> {
//         let mut i = self.best_order_index;

//         loop {
//             let next_order = self.orders[i as usize];

//             if next_order.amount == 0 || self.is_price_better(order, next_order) {
//                 return Some(next_order);
//             }

//             if next_order.next_index == NULL {
//                 return None;
//             } else {
//                 i = next_order.next_index;
//             }
//         }
//     }


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
    pub next_index: u8,
    pub prev_index: u8,
    pub market_maker_index: u8,
    pub _padding: [u8; 5],
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
