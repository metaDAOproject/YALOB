use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use solana_program::clock::Clock;
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod error;
pub mod ix;
pub mod state;
pub mod token_utils;

use crate::error::CLOBError;
use crate::ix::*;
use crate::state::*;
use crate::token_utils::{token_transfer, token_transfer_signed};

pub const PRICE_PRECISION: u128 = 1_000_000_000;
pub const MAX_BPS: u16 = 10_000;

#[program]
pub mod clob {
    use super::*;

    pub fn initialize_global_state(
        ctx: Context<InitializeGlobalState>,
        fee_collector: Pubkey,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        global_state.fee_collector = fee_collector;
        global_state.taker_fee_in_bps = 10;
        global_state.market_maker_burn_in_lamports = 1_000_000_000;

        Ok(())
    }

    pub fn initialize_order_book(ctx: Context<InitializeOrderBook>) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_init()?;

        order_book.base = ctx.accounts.base.key();
        order_book.quote = ctx.accounts.quote.key();

        order_book.base_vault = ctx.accounts.base_vault.key();
        order_book.quote_vault = ctx.accounts.quote_vault.key();

        order_book.buys.side = Side::Buy.into();
        order_book.buys.free_bitmap = FreeBitmap::default();
        order_book.buys.best_order_idx = NULL;
        order_book.buys.worst_order_idx = NULL;

        order_book.sells.side = Side::Sell.into();
        order_book.sells.free_bitmap = FreeBitmap::default();
        order_book.sells.best_order_idx = NULL;
        order_book.sells.worst_order_idx = NULL;

        // TODO: make this configurable via global state
        order_book.twap_oracle.max_observation_change_per_update_bps = 250;
        order_book.twap_oracle.max_observation_change_per_slot_bps = 100;

        order_book.base_fees_sweepable = 0;
        order_book.quote_fees_sweepable = 0;

        order_book.pda_bump = *ctx.bumps.get("order_book").unwrap();

        Ok(())
    }

    pub fn sweep_fees(ctx: Context<SweepFees>) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_mut()?;

        let base_amount = order_book.base_fees_sweepable;
        let quote_amount = order_book.quote_fees_sweepable;

        order_book.base_fees_sweepable = 0;
        order_book.quote_fees_sweepable = 0;

        // Copy these onto the stack before we drop `order_book`
        let base = order_book.base;
        let quote = order_book.quote;
        let pda_bump = order_book.pda_bump;

        let seeds = &[b"order_book", base.as_ref(), quote.as_ref(), &[pda_bump]];

        drop(order_book);

        token_transfer_signed(
            base_amount,
            &ctx.accounts.token_program,
            &ctx.accounts.base_vault,
            &ctx.accounts.base_to,
            &ctx.accounts.order_book,
            seeds,
        )?;

        token_transfer_signed(
            quote_amount,
            &ctx.accounts.token_program,
            &ctx.accounts.quote_vault,
            &ctx.accounts.quote_to,
            &ctx.accounts.order_book,
            seeds,
        )
    }

    // TODO: make it so that after one market maker has been added, we have to
    // wait a configurable cooldown before we can add another
    pub fn add_market_maker(
        ctx: Context<AddMarketMaker>,
        market_maker: Pubkey,
        index: u32,
    ) -> Result<()> {
        let global_state = &ctx.accounts.global_state;
        let mut order_book = ctx.accounts.order_book.load_mut()?;

        require!(
            order_book.market_makers[index as usize].authority == Pubkey::default(),
            CLOBError::IndexAlreadyTaken
        );

        let from = ctx.accounts.payer.key();
        let to = global_state.fee_collector;
        let lamports_to_burn = global_state.market_maker_burn_in_lamports;

        solana_program::program::invoke(
            &solana_program::system_instruction::transfer(&from, &to, lamports_to_burn),
            &[
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.fee_collector.to_account_info(),
            ],
        )?;

        order_book.market_makers[index as usize].authority = market_maker;

        Ok(())
    }

    pub fn top_up_balance(
        ctx: Context<TopUpBalance>,
        market_maker_index: u32,
        base_amount: u64,
        quote_amount: u64,
    ) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_mut()?;

        let market_maker = &mut order_book.market_makers[market_maker_index as usize];

        token_transfer(
            base_amount,
            &ctx.accounts.token_program,
            &ctx.accounts.base_from,
            &ctx.accounts.base_vault,
            &ctx.accounts.authority,
        )?;

        market_maker.base_balance += base_amount;

        token_transfer(
            quote_amount,
            &ctx.accounts.token_program,
            &ctx.accounts.quote_from,
            &ctx.accounts.quote_vault,
            &ctx.accounts.authority,
        )?;

        market_maker.quote_balance += quote_amount;

        Ok(())
    }

    pub fn withdraw_balance(
        ctx: Context<WithdrawBalance>,
        market_maker_index: u32,
        base_amount: u64,
        quote_amount: u64,
    ) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_mut()?;

        let market_maker = &mut order_book.market_makers[market_maker_index as usize];

        require!(
            market_maker.authority == ctx.accounts.authority.key(),
            CLOBError::UnauthorizedMarketMaker
        );

        // These debits cannot be inside the `if` blocks because we drop `order_book`
        market_maker.base_balance = market_maker
            .base_balance
            .checked_sub(base_amount)
            .ok_or(CLOBError::InsufficientBalance)?;

        market_maker.quote_balance = market_maker
            .quote_balance
            .checked_sub(quote_amount)
            .ok_or(CLOBError::InsufficientBalance)?;

        // Copy these onto the stack before we drop `order_book`
        let base = order_book.base;
        let quote = order_book.quote;
        let pda_bump = order_book.pda_bump;

        let seeds = &[b"order_book", base.as_ref(), quote.as_ref(), &[pda_bump]];

        drop(order_book);

        token_transfer_signed(
            base_amount,
            &ctx.accounts.token_program,
            &ctx.accounts.base_vault,
            &ctx.accounts.base_to,
            &ctx.accounts.order_book,
            seeds,
        )?;

        token_transfer_signed(
            quote_amount,
            &ctx.accounts.token_program,
            &ctx.accounts.quote_vault,
            &ctx.accounts.quote_to,
            &ctx.accounts.order_book,
            seeds,
        )
    }

    pub fn submit_limit_order(
        ctx: Context<SubmitLimitOrder>,
        side: Side,
        amount_in: u64,
        price: u64,
        ref_id: u32,
        market_maker_index: u8,
    ) -> Result<u8> {
        let mut order_book = ctx.accounts.order_book.load_mut()?;

        order_book.update_twap_oracle()?;

        let market_maker = order_book.market_makers[market_maker_index as usize];

        require!(
            market_maker.authority == ctx.accounts.authority.key(),
            CLOBError::UnauthorizedMarketMaker
        );

        let (order_list, makers) = order_book.order_list(side);

        let order_idx =
            order_list.insert_order(amount_in, price, ref_id, market_maker_index, makers);

        order_idx.ok_or_else(|| error!(CLOBError::InferiorPrice))
    }

    pub fn cancel_limit_order(
        ctx: Context<CancelLimitOrder>,
        side: Side,
        order_index: u8,
        market_maker_index: u8,
    ) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_mut()?;

        order_book.update_twap_oracle()?;

        let market_maker = order_book.market_makers[market_maker_index as usize];

        require!(
            market_maker.authority == ctx.accounts.authority.key(),
            CLOBError::UnauthorizedMarketMaker
        );

        let (order_list, makers) = order_book.order_list(side);

        let order = order_list.orders[order_index as usize];

        require!(
            order.market_maker_index == market_maker_index,
            CLOBError::UnauthorizedMarketMaker
        );

        order_list.delete_order(order_index, makers);

        Ok(())
    }

    pub fn submit_take_order(
        ctx: Context<SubmitTakeOrder>,
        side: Side,
        amount_in: u64,
        min_out: u64,
    ) -> Result<()> {
        // TODO: add cluster restart logic, preventing take orders within x
        // slots of restart

        assert!(amount_in > 0);

        let global_state = &ctx.accounts.global_state;

        let mut amount_in_after_fees = ((amount_in as u128)
            * (MAX_BPS - global_state.taker_fee_in_bps) as u128)
            / MAX_BPS as u128;

        let mut order_book = ctx.accounts.order_book.load_mut()?;

        order_book.update_twap_oracle()?;

        let (receiving_vault, sending_vault, user_from, user_to) = match side {
            Side::Buy => {
                order_book.quote_fees_sweepable += amount_in - amount_in_after_fees as u64;
                (
                    &ctx.accounts.quote_vault,
                    &ctx.accounts.base_vault,
                    &ctx.accounts.user_quote_account,
                    &ctx.accounts.user_base_account,
                )
            }
            Side::Sell => {
                order_book.base_fees_sweepable += amount_in - amount_in_after_fees as u64;
                (
                    &ctx.accounts.base_vault,
                    &ctx.accounts.quote_vault,
                    &ctx.accounts.user_base_account,
                    &ctx.accounts.user_quote_account,
                )
            }
        };

        token_transfer(
            amount_in,
            &ctx.accounts.token_program,
            &user_from,
            &receiving_vault,
            &ctx.accounts.authority,
        )?;

        let mut amount_out = 0;
        // We cannot delete the orders inside the loop because
        // `order_list.iter()` holds an immutable borrow to the order list.
        let mut filled_orders = Vec::new();

        // If the user is buying, the maker is selling. If the maker is
        // selling, the user is buying.
        let (order_list, makers) = order_book.opposing_order_list(side);

        for (book_order, book_order_idx) in order_list.iter() {
            let order_amount_available = book_order.amount_in as u128; // u128s prevent overflow
            let order_price = book_order.price as u128;

            // If an order is selling 10 BONK at a price of 2 USDC per BONK,
            // the order can take up to 5 USDC (10 / 2). If an order is buying
            // BONK with 10 USDC at a price of 2 USDC per BONK, the order can
            // take up to 20 BONK (10 * 2).
            let amount_order_can_absorb = match side {
                Side::Buy => (order_amount_available * PRICE_PRECISION) / order_price,
                Side::Sell => (order_amount_available * order_price) / PRICE_PRECISION,
            };

            // Can the book order absorb all of a user's input token?
            if amount_order_can_absorb >= amount_in_after_fees {
                // If an order can absorb 15 USDC at a price of 3 USDC per BONK
                // and a user is buying BONK with 6 USDC, the user should receive
                // 2 BONK (6 / 3).
                //
                // If an order can absorb 20 BONK at a price of 3 USDC per BONK
                // and a user is selling 10 BONK, the user should receive 30
                // USDC (10 * 3).
                let user_to_receive = match side {
                    Side::Buy => (amount_in_after_fees * PRICE_PRECISION) / order_price,
                    Side::Sell => (amount_in_after_fees * order_price) / PRICE_PRECISION,
                } as u64;
                amount_out += user_to_receive;

                order_list.orders[book_order_idx as usize].amount_in -= user_to_receive;

                match side {
                    Side::Buy => {
                        makers[book_order.market_maker_index as usize].quote_balance +=
                            amount_in_after_fees as u64
                    }
                    Side::Sell => {
                        makers[book_order.market_maker_index as usize].base_balance +=
                            amount_in_after_fees as u64
                    }
                };

                break;
            } else {
                amount_in_after_fees -= amount_order_can_absorb;
                amount_out += order_amount_available as u64;

                match side {
                    Side::Buy => {
                        makers[book_order.market_maker_index as usize].quote_balance +=
                            amount_order_can_absorb as u64
                    }
                    Side::Sell => {
                        makers[book_order.market_maker_index as usize].base_balance +=
                            amount_order_can_absorb as u64
                    }
                };

                filled_orders.push(book_order_idx);
            }
        }

        for order_idx in filled_orders {
            order_list.delete_order(order_idx, makers);
        }

        require!(amount_out >= min_out, CLOBError::TakeNotFilled);

        let base = order_book.base;
        let quote = order_book.quote;
        let pda_bump = order_book.pda_bump;

        let seeds = &[b"order_book", base.as_ref(), quote.as_ref(), &[pda_bump]];

        drop(order_book);

        token_transfer_signed(
            amount_out,
            &ctx.accounts.token_program,
            sending_vault,
            user_to,
            &ctx.accounts.order_book,
            seeds,
        )
    }

    /**** GETTERS ****/

    pub fn get_twap(ctx: Context<Getter>) -> Result<TWAPOracle> {
        let order_book = ctx.accounts.order_book.load()?;

        Ok(order_book.twap_oracle)
    }

    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct MarketMakerBalances {
        pub base_balance: u64,
        pub quote_balance: u64,
    }

    pub fn get_market_maker_balances(
        ctx: Context<Getter>,
        maker_pubkey: Pubkey,
    ) -> Result<MarketMakerBalances> {
        let order_book = ctx.accounts.order_book.load()?;
        let makers = &order_book.market_makers;

        for market_maker in makers {
            if market_maker.authority == maker_pubkey {
                return Ok(MarketMakerBalances {
                    base_balance: market_maker.base_balance,
                    quote_balance: market_maker.quote_balance,
                });
            }
        }

        Err(error!(CLOBError::MakerNotFound))
    }

    pub fn get_order_index(
        ctx: Context<Getter>,
        side: Side,
        ref_id: u32,
        market_maker_index: u8,
    ) -> Result<Option<u8>> {
        let order_book = ctx.accounts.order_book.load()?;
        let order_list = match side {
            Side::Buy => order_book.buys,
            Side::Sell => order_book.sells,
        };

        for (order, order_idx) in order_list.iter() {
            if order.ref_id == ref_id && order.market_maker_index == market_maker_index {
                return Ok(Some(order_idx));
            }
        }

        Ok(None)
    }

    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AmountAndPrice {
        pub amount: u64,
        pub price: u64,
    }

    pub fn get_best_orders(ctx: Context<Getter>, side: Side) -> Result<Vec<AmountAndPrice>> {
        let order_book = ctx.accounts.order_book.load()?;
        let order_list = match side {
            Side::Buy => order_book.buys,
            Side::Sell => order_book.sells,
        };

        let max_returnable = (solana_program::program::MAX_RETURN_DATA - size_of::<u32>())
            / size_of::<AmountAndPrice>();

        let mut orders = Vec::with_capacity(max_returnable);

        for (order, _) in order_list.iter() {
            orders.push(AmountAndPrice {
                amount: order.amount_in,
                price: order.price,
            });

            if orders.len() == max_returnable {
                break;
            }
        }

        Ok(orders)
    }
}
