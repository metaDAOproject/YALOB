use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod ix;
pub mod state;

use crate::ix::*;
use crate::state::OrderBook;

#[program]
pub mod clob {
    use super::*;

    pub fn initialize_order_book(ctx: Context<InitializeOrderBook>) -> Result<()> {
        let mut order_book = ctx.accounts.order_book.load_init()?;
        Ok(())
    }
}
