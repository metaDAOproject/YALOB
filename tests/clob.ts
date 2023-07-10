import * as anchor from "@coral-xyz/anchor";
import * as token from "@solana/spl-token";

import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

describe("CLOB", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet.payer;
  const connection = provider.connection;

  const program = anchor.workspace.Clob as Program<Clob>;

  it("Passes tests", async () => {
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

    const marketMaker = anchor.web3.Keypair.generate();

    const mmBase = await token.createAccount(
      connection,
      payer,
      base,
      marketMaker.publicKey
    );

    const mmQuote = await token.createAccount(
      connection,
      payer,
      quote,
      marketMaker.publicKey
    );

    await token.mintTo(connection, payer, base, mmBase, mintAuthority, 100000);
    await token.mintTo(
      connection,
      payer,
      quote,
      mmQuote,
      mintAuthority,
      100000
    );

    await program.methods
      .addMarketMaker(marketMaker.publicKey, 0)
      .accounts({
        orderBook,
        payer: payer.publicKey,
        globalState,
        feeCollector: feeCollector.publicKey,
      })
      .rpc();

    await program.methods
      .topUpBalance(0, new anchor.BN(10_000), new anchor.BN(100_000))
      .accounts({
        authority: marketMaker.publicKey,
        orderBook,
        baseFrom: mmBase,
        quoteFrom: mmQuote,
        baseVault,
        quoteVault,
        tokenProgam: token.TOKEN_PROGRAM_ID,
      })
      .signers([marketMaker])
      .rpc();

    await program.methods
      .submitLimitOrder(
        { buy: {} },
        new anchor.BN(100),
        new anchor.BN(1e9),
        12,
        0
      )
      .accounts({
        authority: marketMaker.publicKey,
        orderBook,
      })
      .signers([marketMaker])
      .rpc();

    await program.methods
      .withdrawBalance(0, new anchor.BN(1000), new anchor.BN(0))
      .accounts({
        authority: marketMaker.publicKey,
        orderBook,
        baseTo: mmBase,
        quoteTo: mmQuote,
        baseVault,
        quoteVault,
        tokenProgram: token.TOKEN_PROGRAM_ID,
      })
      .signers([marketMaker])
      .rpc();

    console.log(await token.getAccount(connection, mmBase));

    for (let i = 0; i < 10; i++) {
      await program.methods
        .submitLimitOrder(
          { sell: {} },
          new anchor.BN(101),
          new anchor.BN(1e9 + 1),
          13,
          0
        )
        .accounts({
          authority: marketMaker.publicKey,
          orderBook,
        })
        .signers([marketMaker])
        .rpc();
    }

    let orderIndex = await program.methods
      .getOrderIndex({ buy: {} }, 12, 0)
      .accounts({
        orderBook,
      })
      .view();

    await program.methods
      .cancelLimitOrder({ buy: {} }, orderIndex, 0)
      .accounts({
        orderBook,
        authority: marketMaker.publicKey,
      })
      .signers([marketMaker])
      .rpc();

    let orders = await program.methods
      .getBestOrders({ buy: {} })
      .accounts({
        orderBook,
      })
      .view();

    console.log(orders);

    await program.methods
      .submitLimitOrder(
        { buy: {} },
        new anchor.BN(100),
        new anchor.BN(1e9),
        12,
        0
      )
      .accounts({
        authority: marketMaker.publicKey,
        orderBook,
      })
      .signers([marketMaker])
      .rpc();

    orders = await program.methods
      .getBestOrders({ buy: {} })
      .accounts({
        orderBook,
      })
      .view();

    console.log(orders);

    orderIndex = await program.methods
      .getOrderIndex({ buy: {} }, 12, 0)
      .accounts({
        orderBook,
      })
      .view();

    orders = await program.methods
      .getBestOrders({ buy: {} })
      .accounts({
        orderBook,
      })
      .view();

    console.log(orders);

    await program.methods
      .submitTakeOrder({ sell: {} }, new anchor.BN(500), new anchor.BN(1))
      .accounts({
        orderBook,
        authority: marketMaker.publicKey,
        userBaseAccount: mmBase,
        userQuoteAccount: mmQuote,
        globalState,
        baseVault,
        quoteVault,
        tokenProgram: token.TOKEN_PROGRAM_ID,
      })
      .signers([marketMaker])
      .rpc();

    orders = await program.methods
      .getBestOrders({ buy: {} })
      .accounts({
        orderBook,
      })
      .view();

    console.log(orders);

    // let ix = await program.methods.getOrders({buy: {}})
    //   .accounts({orderBook})
    //   .instruction();
    // let tx = new anchor.web3.Transaction();
    // tx.add(ix);

    // let res = await connection.simulateTransaction(tx, [payer]);
    // console.log(res.value.returnData.data);

    // const buf = Buffer.from(res.value.returnData.data[0], 'base64');

    // console.log(program.coder.types.decode("ClientOrder", buf));

    let ob = await program.account.orderBook.fetch(orderBook);

    console.log(ob.twapOracle);

    // console.log(ob.buys);
    // console.log(ob.marketMakers);
  });
});
