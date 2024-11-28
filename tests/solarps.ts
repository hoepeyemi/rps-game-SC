import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Solarps } from "../target/types/solarps";
import { TransactionMessage, SystemProgram, VersionedTransaction, PublicKey } from "@solana/web3.js";

describe("solarps", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const connection = provider.connection;
  const commitment = "processed";
  // console.log("Connection: ", connection);

  const program = anchor.workspace.Solarps as Program<Solarps>;

  // variables
  const admin = anchor.web3.Keypair.generate();
  const user = anchor.web3.Keypair.generate();
  const treasury_wallet = anchor.web3.Keypair.generate();
  
  const pyth_account = new PublicKey("");

  let deposit_amount = 5_000_000_000;
  let bet_amount = 2_000_000_000;
  let win_percentage = [33, 66, 99];
  let reward_policy = [10, 0, 0];

  let global_state: PublicKey;
  let user_state: PublicKey;
  let vault: PublicKey;

  const GLOBAL_STATE_SEED = "GLOBAL-STATE-SEED";
  const USER_STATE_SEED = "USER-STATE-SEED";
  const VAULT_SEED = "VAULT_SEED";

  it("Is initialized!", async () => {
    // 1. Airdrop 100 SOL to admin
    const signature = await provider.connection.requestAirdrop(admin.publicKey, 100_000_000_000);
    const latestBlockhash = await connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      {
        signature,
        ...latestBlockhash,
      },
      commitment
    );

    // 2. Fund main roles: admin and user
    const fundingTxMessageV0 = new TransactionMessage({
      payerKey: admin.publicKey,
      recentBlockhash: latestBlockhash.blockhash,
      instructions: [
        SystemProgram.transfer({
          fromPubkey: admin.publicKey,
          toPubkey: user.publicKey,
          lamports: 9_000_000_000,
        })
      ],
    }).compileToV0Message();
    const fundingTx = new VersionedTransaction(fundingTxMessageV0);
    fundingTx.sign([admin]);
    const result = await connection.sendRawTransaction(fundingTx.serialize());

    global_state = PublicKey.findProgramAddressSync([
      Buffer.from(anchor.utils.bytes.utf8.encode(GLOBAL_STATE_SEED)),
      admin.publicKey.toBytes()], program.programId)[0];
    vault = PublicKey.findProgramAddressSync([Buffer.from(anchor.utils.bytes.utf8.encode(VAULT_SEED))], program.programId)[0];
    // Add your test here.
    const tx = await program.methods.initialize().accounts({
      admin: admin.publicKey,
      globalState: global_state,
      vault: vault,
      systemProgram: SystemProgram.programId,
    }).signers([admin]).rpc();
  });

  it("Set Operator", async () => {
    const tx = await program.methods.setOperator(treasury_wallet.publicKey).accounts({
      admin: admin.publicKey,
      globalState: global_state,
    }).signers([admin]).rpc();
  });

  it("Set Info", async () => {
    const tx = await program.methods.setInfo(treasury_wallet.publicKey, new anchor.BN(5), false).accounts({
      operator: treasury_wallet.publicKey,
      globalState: global_state,
    }).signers([treasury_wallet]).rpc();
  });

  it("Coin Flip", async () => {
    user_state = PublicKey.findProgramAddressSync([Buffer.from(anchor.utils.bytes.utf8.encode(USER_STATE_SEED)), user.publicKey.toBytes()], program.programId)[0];
    let _globalState = await program.account.globalState.fetch(global_state);
    console.log("Treasury Fee: ", _globalState.treasuryFee.toNumber());

    const tx = await program.methods.coinflip(new anchor.BN(bet_amount)).accounts({
      globalState: global_state,
      pythAccount: pyth_account,
      treasuryAccount: treasury_wallet.publicKey,
      user: user.publicKey,
      userState: user_state,
      vault: vault,
      systemProgram: SystemProgram.programId,
    }).signers([user]).rpc();
    let _user_state = await program.account.userState.fetch(user_state);
    console.log("user address : ", _user_state.user.toBase58());
    console.log("reward amount : ", _user_state.rewardAmount.toNumber());
    console.log("last spin result : ", _user_state.lastSpinresult);

    let treasuryAccount = await provider.connection.getAccountInfo(treasury_wallet.publicKey);
    console.log("Treasury: ", treasuryAccount.lamports);
  });
});
