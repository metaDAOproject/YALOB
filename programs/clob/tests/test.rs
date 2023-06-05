pub use solana_program_test::{ProgramTest, tokio};
use solana_sdk::transaction::{Transaction};
use anchor_lang::prelude::*;
use clob::{self, clob::initialize_global_state};

#[tokio::test]
async fn test() {
    let test = ProgramTest::new("clob", clob::ID, None);

    let context = test.start_with_context().await;

    let payer = context.payer;

    let global_state = Pubkey::find_program_address(
        &[b"WWCACOTMICMIBMHAFTTWYGHMB".as_ref()],
        &clob::ID,
    )
    .0;

    // let ix = clob::ix::InitializeGlobalState {
    //     global_state,
    //     payer,
    //     system_program,
    // }
    // initialize_global_state(ctx, fee_collector)

    // let transaction = Transaction::new(from_keypairs, message, recent_blockhash);

    // context.banks_client.process_transaction(transaction).await.unwrap();
}
