use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod ix;
pub mod state;

use crate::ix::*;
use crate::state::*;

#[program]
pub mod clob {
    use super::*;

    pub fn initialize_global_state(ctx: Context<InitializeGlobalState>, fee_admin: Pubkey) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        global_state.fee_admin = fee_admin;
        global_state.fee_in_bps = 15;

        Ok(())
    }

    pub fn initialize_order_book(ctx: Context<InitializeOrderBook>) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_init()?;
        Ok(())
    }
}
