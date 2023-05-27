import * as anchor from "@coral-xyz/anchor";
import * as token from "@solana/spl-token";

import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

describe("CLOB", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet.payer;

  const program = anchor.workspace.Clob as Program<Clob>;

  it("Order books can be initialized", async () => {
    const [globalState] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("WWCACOTMICMIBMHAFTTWYGHMB")],
      program.programId,
    );
    const feeAdmin = anchor.web3.Keypair.generate();

    await program.methods.initializeGlobalState(feeAdmin.publicKey)
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
      [anchor.utils.bytes.utf8.encode("order_book"), base.toBuffer(), quote.toBuffer()],
      program.programId,
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

    await program.methods.initializeOrderBook()
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
  });
});
