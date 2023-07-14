import * as anchor from "@coral-xyz/anchor";
import * as token from "@solana/spl-token";

import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

import { assert } from "chai";

describe("YALOB", () => {
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

    assert(
      mm0BalsAfter.quoteBalance.eq(
        mm0BalsBefore.quoteBalance.sub(new anchor.BN(100))
      )
    );

    await program.methods
      .submitLimitOrder(
        { buy: {} },
        new anchor.BN(101), // amount
        new anchor.BN(1e9 + 2), // price
        13, // ref id
        1 // mm index
      )
      .accounts({
        authority: mm1.publicKey,
        orderBook,
      })
      .signers([mm1])
      .rpc();

    await program.methods
      .submitLimitOrder(
        { buy: {} },
        new anchor.BN(102), // amount
        new anchor.BN(1e9 + 1), // price
        14, // ref id
        1 // mm index
      )
      .accounts({
        authority: mm1.publicKey,
        orderBook,
      })
      .signers([mm1])
      .rpc();

    let buys = await program.methods
      .getBestOrders({ buy: {} })
      .accounts({
        orderBook,
      })
      .view();

    // buys should be ascending price
    assert(buys[0].amount.eq(new anchor.BN(101)));
    assert(buys[1].amount.eq(new anchor.BN(102)));
    assert(buys[2].amount.eq(new anchor.BN(100)));

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
        authority: mm0.publicKey,
      })
      .signers([mm0])
      .rpc();

    mm0BalsAfter = await program.methods
      .getMarketMakerBalances(mm0.publicKey)
      .accounts({
        orderBook,
      })
      .view();

    // should get their tokens back
    assert(mm0BalsAfter.quoteBalance.eq(mm0BalsBefore.quoteBalance));

    await program.methods
      .submitLimitOrder(
        { sell: {} },
        new anchor.BN(300), // amount
        new anchor.BN(2e9), // price
        15, // ref id
        0 // mm index
      )
      .accounts({
        authority: mm0.publicKey,
        orderBook,
      })
      .signers([mm0])
      .rpc();

    mm0BalsAfter = await program.methods
      .getMarketMakerBalances(mm0.publicKey)
      .accounts({
        orderBook,
      })
      .view();

    assert(
      mm0BalsAfter.baseBalance.eq(
        mm0BalsBefore.baseBalance.sub(new anchor.BN(300))
      )
    );

    // the limit order is for 300 at a price of 2, therefore 50 should cost 100
    await program.methods
      .submitTakeOrder(
        { buy: {} },
        new anchor.BN(100), 
        new anchor.BN(49), // allow round down to 49
      )
      .accounts({
	globalState,
	userBaseAccount: mm1Base,
	userQuoteAccount: mm1Quote,
	baseVault,
	quoteVault,
        authority: mm1.publicKey,
        orderBook,
	tokenProgram: token.TOKEN_PROGRAM_ID,
      })
      .signers([mm1])
      .rpc();
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
    BASE_AMOUNT * 2
  );

  await token.mintTo(
    connection,
    payer,
    quote,
    mmQuote,
    mintAuthority,
    QUOTE_AMOUNT * 2
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
    .topUpBalance(
      index,
      new anchor.BN(BASE_AMOUNT),
      new anchor.BN(QUOTE_AMOUNT)
    )
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
