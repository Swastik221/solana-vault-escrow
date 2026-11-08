use {
  anchor_lang::{
      prelude::Pubkey,
      solana_program::{instruction::Instruction, system_program},
      AccountDeserialize, InstructionData, ToAccountMetas,
  },
  anchor_spl::associated_token,
  litesvm::LiteSVM,
  litesvm_token::{
      get_spl_account, spl_token::state::Account as SplAccount,
      CreateAssociatedTokenAccount, CreateMint, MintTo,
  },
  solana_keypair::Keypair,
  solana_message::{Message, VersionedMessage},
  solana_signer::Signer,
  solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_make_escrow() {
  let program_id = escrow::id();
  let maker = Keypair::new();
  let maker_pubkey = maker.pubkey();

  // Derive the escrow PDA.
  let (escrow_pda, _) = Pubkey::find_program_address(
      &[b"escrow", maker_pubkey.as_ref()],
      &program_id,
  );

  // Start LiteSVM with the built-in SPL programs.
  let mut svm = LiteSVM::new().with_default_programs();

  let bytes =
      include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/escrow.so"));

  svm.add_program(program_id, bytes).unwrap();

  // Fund the maker.
  svm.airdrop(&maker_pubkey, 5_000_000_000).unwrap();

  // Create Token A and Token B mints.
  let mint_a = CreateMint::new(&mut svm, &maker)
      .authority(&maker_pubkey)
      .decimals(6)
      .send()
      .unwrap();

  let mint_b = CreateMint::new(&mut svm, &maker)
      .authority(&maker_pubkey)
      .decimals(6)
      .send()
      .unwrap();

  // Create the maker's Token A associated token account.
  let maker_token_a = CreateAssociatedTokenAccount::new(
      &mut svm,
      &maker,
      &mint_a,
  )
  .send()
  .unwrap();

  // Mint 1,000 Token A (6 decimal places).
  MintTo::new(
      &mut svm,
      &maker,
      &mint_a,
      &maker_token_a,
      1_000_000_000,
  )
  .send()
  .unwrap();

  // Derive the escrow's Token A associated token account.
  let vault = associated_token::get_associated_token_address(
      &escrow_pda,
      &mint_a,
  );

  // Create the make instruction.
  let amount = 500_000_000u64;
  let receive = 250_000_000u64;

  let make_ix = Instruction::new_with_bytes(
      program_id,
      &escrow::instruction::Make { amount, receive }.data(),
      escrow::accounts::Make {
          maker: maker_pubkey,
          escrow: escrow_pda,
          mint_a,
          mint_b,
          maker_token_a,
          vault,
          token_program: anchor_spl::token::ID,
          associated_token_program: associated_token::ID,
          system_program: system_program::ID,
      }
      .to_account_metas(None),
  );

  // Sign and send the transaction.
  let message = Message::new_with_blockhash(
      &[make_ix],
      Some(&maker_pubkey),
      &svm.latest_blockhash(),
  );

  let transaction =
      VersionedTransaction::try_new(
          VersionedMessage::Legacy(message),
          &[&maker],
      )
      .unwrap();

  let result = svm.send_transaction(transaction);

  assert!(
      result.is_ok(),
      "Escrow make instruction failed: {:?}",
      result.err()
  );

  // Verify the stored escrow state.
  let escrow_account = svm.get_account(&escrow_pda).unwrap();
  let mut state_data: &[u8] = &escrow_account.data;

  let state = escrow::state::Escrow::try_deserialize(&mut state_data).unwrap();

  assert_eq!(state.maker, maker_pubkey);
  assert_eq!(state.mint_a, mint_a);
  assert_eq!(state.mint_b, mint_b);
  assert_eq!(state.receive, receive);

  // Verify Token A was transferred into the escrow vault.
  let vault_account =
      get_spl_account::<SplAccount>(&svm, &vault).unwrap();

  assert_eq!(vault_account.amount, amount);

  // Verify the maker's remaining Token A balance.
  let maker_account =
      get_spl_account::<SplAccount>(&svm, &maker_token_a).unwrap();

  assert_eq!(maker_account.amount, 500_000_000);

  println!("Escrow make test passed!");
}