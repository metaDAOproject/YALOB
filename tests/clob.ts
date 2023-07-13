import * as anchor from "@coral-xyz/anchor";
import * as token from "@solana/spl-token";

import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

import { assert } from "chai";

describe("CLOB", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet.payer;
  const connection = provider.connection;

  const program = anchor.workspace.Clob as Program<Clob>;

  it("Can be initialized", async () => {
    const [globalState] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("WWCACOTMICMIBMHAFTTWYGHMB")],
      program.programId
    );
    const feeCollector = anchor.web3.Keypair.generate();

    await program.methods
      .initializeGlobalState(feeCollector.publicKey)
      .accounts({
        globalState,
        payer: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const mintAuthority = anchor.web3.Keypair.generate();
    const quote = await token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      mintAuthority.publicKey,
      8
    );
    const base = await token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      mintAuthority.publicKey,
      8
    );

    const [orderBook] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("order_book"),
        base.toBuffer(),
        quote.toBuffer(),
      ],
      program.programId
    );

    const baseVault = await token.getAssociatedTokenAddress(
      base,
      orderBook,
      true
    );

    const quoteVault = await token.getAssociatedTokenAddress(
      quote,
      orderBook,
      true
    );

    await program.methods
      .initializeOrderBook()
      .accounts({
        orderBook,
        payer: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        base,
        quote,
        baseVault,
        quoteVault,
      })
      .rpc();

    const [mm0, mm0Base, mm0Quote] = await generateMarketMaker(
      0, // reside at 0th index
      program,
      connection,
      payer,
      globalState,
      orderBook,
      baseVault,
      quoteVault,
      base,
      quote,
      mintAuthority,
      feeCollector
    );

    const [mm1, mm1Base, mm1Quote] = await generateMarketMaker(
      1, // reside at 1st index
      program,
      connection,
      payer,
      globalState,
      orderBook,
      baseVault,
      quoteVault,
      base,
      quote,
      mintAuthority,
      feeCollector
    );

    let mm0BalsBefore = await program.methods
      .getMarketMakerBalances(mm0.publicKey)
      .accounts({
        orderBook,
      })
      .view();

    await program.methods
      .submitLimitOrder(
        { buy: {} },
        new anchor.BN(100), // amount
        new anchor.BN(1e9), // price
        12, // ref id
        0 // mm index
      )
      .accounts({
        authority: mm0.publicKey,
        orderBook,
      })
      .signers([mm0])
      .rpc();

    let mm0BalsAfter = await program.methods
      .getMarketMakerBalances(mm0.publicKey)
      .accounts({
        orderBook,
      })
      .view();

    assert.equal(
      mm0BalsAfter.quoteBalance,
        mm0BalsBefore.quoteBalance - new anchor.BN(100)
    );

    //await program.methods
    //  .withdrawBalance(0, new anchor.BN(1000), new anchor.BN(0))
    //  .accounts({
    //    authority: mm0.publicKey,
    //    orderBook,
    //    baseTo: mm0Base,
    //    quoteTo: mm0Quote,
    //    baseVault,
    //    quoteVault,
    //    tokenProgram: token.TOKEN_PROGRAM_ID,
    //  })
    //  .signers([mm0])
    //  .rpc();

    // console.log(await token.getAccount(connection, mmBase));

    // for (let i = 0; i < 10; i++) {
    //   await program.methods
    //     .submitLimitOrder(
    //       { sell: {} },
    //       new anchor.BN(101),
    //       new anchor.BN(1e9 + 1),
    //       13,
    //       0
    //     )
    //     .accounts({
    //       authority: marketMaker.publicKey,
    //       orderBook,
    //     })
    //     .signers([marketMaker])
    //     .rpc();
    // }

    // let orderIndex = await program.methods
    //   .getOrderIndex({ buy: {} }, 12, 0)
    //   .accounts({
    //     orderBook,
    //   })
    //   .view();

    // await program.methods
    //   .cancelLimitOrder({ buy: {} }, orderIndex, 0)
    //   .accounts({
    //     orderBook,
    //     authority: marketMaker.publicKey,
    //   })
    //   .signers([marketMaker])
    //   .rpc();

    // let orders = await program.methods
    //   .getBestOrders({ buy: {} })
    //   .accounts({
    //     orderBook,
    //   })
    //   .view();

    // console.log(orders);

    // await program.methods
    //   .submitLimitOrder(
    //     { buy: {} },
    //     new anchor.BN(100),
    //     new anchor.BN(1e9),
    //     12,
    //     0
    //   )
    //   .accounts({
    //     authority: marketMaker.publicKey,
    //     orderBook,
    //   })
    //   .signers([marketMaker])
    //   .rpc();

    // orders = await program.methods
    //   .getBestOrders({ buy: {} })
    //   .accounts({
    //     orderBook,
    //   })
    //   .view();

    // console.log(orders);

    // orderIndex = await program.methods
    //   .getOrderIndex({ buy: {} }, 12, 0)
    //   .accounts({
    //     orderBook,
    //   })
    //   .view();

    // orders = await program.methods
    //   .getBestOrders({ buy: {} })
    //   .accounts({
    //     orderBook,
    //   })
    //   .view();

    // console.log(orders);

    // await program.methods
    //   .submitTakeOrder({ sell: {} }, new anchor.BN(500), new anchor.BN(1))
    //   .accounts({
    //     orderBook,
    //     authority: marketMaker.publicKey,
    //     userBaseAccount: mmBase,
    //     userQuoteAccount: mmQuote,
    //     globalState,
    //     baseVault,
    //     quoteVault,
    //     tokenProgram: token.TOKEN_PROGRAM_ID,
    //   })
    //   .signers([marketMaker])
    //   .rpc();

    // orders = await program.methods
    //   .getBestOrders({ buy: {} })
    //   .accounts({
    //     orderBook,
    //   })
    //   .view();

    // console.log(orders);

    // // let ix = await program.methods.getOrders({buy: {}})
    // //   .accounts({orderBook})
    // //   .instruction();
    // // let tx = new anchor.web3.Transaction();
    // // tx.add(ix);

    // // let res = await connection.simulateTransaction(tx, [payer]);
    // // console.log(res.value.returnData.data);

    // // const buf = Buffer.from(res.value.returnData.data[0], 'base64');

    // // console.log(program.coder.types.decode("ClientOrder", buf));

    // let ob = await program.account.orderBook.fetch(orderBook);

    // console.log(ob.twapOracle);

    // // console.log(ob.buys);
    // // console.log(ob.marketMakers);
  });
});

const BASE_AMOUNT = 1_000_000_000;
const QUOTE_AMOUNT = 1_000_000_000;

async function generateMarketMaker(
  index: number,
  program: Program<Clob>,
  connection: anchor.Connection,
  payer: anchor.web3.Keypair,
  globalState: anchor.web3.PublicKey,
  orderBook: anchor.web3.PublicKey,
  baseVault: anchor.web3.PublicKey,
  quoteVault: anchor.web3.PublicKey,
  base: anchor.web3.PublicKey,
  quote: anchor.web3.PublicKey,
  mintAuthority: anchor.web3.Keypair,
  feeCollector: anchor.web3.Keypair
): [anchor.web3.Keypair, anchor.web3.PublicKey, anchor.web3.PublicKey] {
  const mm = anchor.web3.Keypair.generate();

  const mmBase = await token.createAccount(
    connection,
    payer,
    base,
    mm.publicKey
  );

  const mmQuote = await token.createAccount(
    connection,
    payer,
    quote,
    mm.publicKey
  );

  await token.mintTo(
    connection,
    payer,
    base,
    mmBase,
    mintAuthority,
    BASE_AMOUNT
  );

  await token.mintTo(
    connection,
    payer,
    quote,
    mmQuote,
    mintAuthority,
    QUOTE_AMOUNT
  );

  await program.methods
    .addMarketMaker(mm.publicKey, index)
    .accounts({
      orderBook,
      payer: payer.publicKey,
      globalState,
      feeCollector: feeCollector.publicKey,
    })
    .rpc();

  await program.methods
    .topUpBalance(0, new anchor.BN(BASE_AMOUNT), new anchor.BN(QUOTE_AMOUNT))
    .accounts({
      orderBook,
      authority: mm.publicKey,
      baseFrom: mmBase,
      quoteFrom: mmQuote,
      baseVault,
      quoteVault,
      tokenProgram: token.TOKEN_PROGRAM_ID,
    })
    .signers([mm])
    .rpc();

  return [mm, mmBase, mmQuote];
}
