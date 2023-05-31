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
    await token.mintTo(connection, payer, quote, mmQuote, mintAuthority, 10000);

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
      .topUpBalance(0, new anchor.BN(10_000), new anchor.BN(1_000))
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
  });
});
