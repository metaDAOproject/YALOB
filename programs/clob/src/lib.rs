use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_spl::token;
use bytemuck::Zeroable;

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

        order_book.base_vault = ctx.accounts.base_vault.key();
        order_book.quote_vault = ctx.accounts.quote_vault.key();

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
            order_book.market_makers[index as usize].authority == Pubkey::zeroed(),
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
        }

        let market_maker = &mut order_book.market_makers[market_maker_index as usize];

        market_maker.base_balance += base_amount;
        market_maker.quote_balance += quote_amount;

        Ok(())
    }
}
