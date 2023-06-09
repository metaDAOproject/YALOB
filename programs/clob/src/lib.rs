use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_spl::token;
// use borsh::{BorshDeserialize, BorshSerialize};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod error;
pub mod ix;
pub mod state;

use crate::error::CLOBError;
use crate::ix::*;
use crate::state::*;

#[program]
pub mod clob {
    use super::*;

    pub fn initialize_global_state(
        ctx: Context<InitializeGlobalState>,
        fee_collector: Pubkey,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        global_state.fee_collector = fee_collector;
        global_state.fee_in_bps = 15;
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

        order_book.pda_bump = *ctx.bumps.get("order_book").unwrap();

        Ok(())
    }

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

        if base_amount > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_progam.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.base_from.to_account_info(),
                        to: ctx.accounts.base_vault.to_account_info(),
                        authority: ctx.accounts.authority.to_account_info(),
                    },
                ),
                base_amount,
            )?;

            market_maker.base_balance += base_amount;
        }

        if quote_amount > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_progam.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.quote_from.to_account_info(),
                        to: ctx.accounts.quote_vault.to_account_info(),
                        authority: ctx.accounts.authority.to_account_info(),
                    },
                ),
                quote_amount,
            )?;

            market_maker.quote_balance += quote_amount;
        }

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

        let seeds = &[
            b"order_book",
            base.as_ref(),
            quote.as_ref(),
            &[pda_bump]
        ];

        drop(order_book);

        if base_amount > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_progam.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.base_vault.to_account_info(),
                        to: ctx.accounts.base_to.to_account_info(),
                        authority: ctx.accounts.order_book.to_account_info(),
                    },
                    &[seeds]
                ),
                base_amount,
            )?;
        }

        if quote_amount > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_progam.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.quote_vault.to_account_info(),
                        to: ctx.accounts.quote_to.to_account_info(),
                        authority: ctx.accounts.order_book.to_account_info(),
                    },
                    &[seeds]
                ),
                base_amount,
            )?;
        }

        Ok(())
    }

    pub fn submit_limit_order(
        ctx: Context<SubmitLimitOrder>,
        side: Side,
        amount_in: u64,
        price: u64,
        market_maker_index: u8,
    ) -> Result<()> {
        // TODO: add cluster restart logic, preventing take orders within x
        // slots of restart

        let mut order_book = ctx.accounts.order_book.load_mut()?;

        let market_maker = &mut order_book.market_makers[market_maker_index as usize];

        require!(
            market_maker.authority == ctx.accounts.authority.key(),
            CLOBError::UnauthorizedMarketMaker
        );

        match side {
            Side::Buy => {
                market_maker.quote_balance = market_maker
                    .quote_balance
                    .checked_sub(amount_in)
                    .ok_or(CLOBError::InsufficientBalance)?;

                order_book
                    .buys
                    .insert_order(amount_in, price, market_maker_index);
            }
            Side::Sell => {
                market_maker.base_balance = market_maker
                    .base_balance
                    .checked_sub(amount_in)
                    .ok_or(CLOBError::InsufficientBalance)?;

                order_book
                    .sells
                    .insert_order(amount_in, price, market_maker_index);
            }
        }

        Ok(())
    }
}
