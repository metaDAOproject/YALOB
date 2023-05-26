import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

describe("CLOB", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet.payer;

  const program = anchor.workspace.Clob as Program<Clob>;

  it("Order books can be initialized", async () => {
    const [orderBook] = anchor.web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("order_book")],
      program.programId,
    );

    await program.methods.initializeOrderBook()
      .accounts({
        orderBook,
        payer: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
  });
});
