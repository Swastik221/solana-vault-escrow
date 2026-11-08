
use {
  anchor_lang::{
      prelude::Pubkey,
      solana_program::{instruction::Instruction, system_program},
      AccountDeserialize, InstructionData, ToAccountMetas,
  },
  anchor_spl::associated_token,
  litesvm::LiteSVM,
  litesvm_token::{
      get_spl_account,
      spl_token::state::Account as SplAccount,
      CreateAssociatedTokenAccount, CreateMint, MintTo,
  },
  solana_keypair::Keypair,
  solana_message::{Message, VersionedMessage},
  solana_signer::Signer,
  solana_transaction::versioned::VersionedTransaction,
};

fn setup_svm() -> LiteSVM {
  LiteSVM::new().with_default_programs()
}

fn load_escrow_program(svm: &mut LiteSVM) {
  let program_id = escrow::id();

  let bytes =
      include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/escrow.so"));

  svm.add_program(program_id, bytes).unwrap();
}

#[test]
fn test_make_escrow() {
  let program_id = escrow::id();
  let maker = Keypair::new();
  let maker_pubkey = maker.pubkey();

  let (escrow_pda, _) = Pubkey::find_program_address(
      &[b"escrow", maker_pubkey.as_ref()],
      &program_id,
  );

  let mut svm = setup_svm();
  load_escrow_program(&mut svm);

  svm.airdrop(&maker_pubkey, 5_000_000_000).unwrap();

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

  let maker_token_a =
      CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
          .send()
          .unwrap();

  MintTo::new(
      &mut svm,
      &maker,
      &mint_a,
      &maker_token_a,
      1_000_000_000,
  )
  .send()
  .unwrap();

  let vault =
      associated_token::get_associated_token_address(&escrow_pda, &mint_a);

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

  let message = Message::new_with_blockhash(
      &[make_ix],
      Some(&maker_pubkey),
      &svm.latest_blockhash(),
  );

  let transaction = VersionedTransaction::try_new(
      VersionedMessage::Legacy(message),
      &[&maker],
  )
  .unwrap();

  svm.send_transaction(transaction)
      .expect("Make instruction failed");

  let escrow_account = svm.get_account(&escrow_pda).unwrap();
  let mut state_data: &[u8] = &escrow_account.data;

  let state =
      escrow::state::Escrow::try_deserialize(&mut state_data).unwrap();

  assert_eq!(state.maker, maker_pubkey);
  assert_eq!(state.mint_a, mint_a);
  assert_eq!(state.mint_b, mint_b);
  assert_eq!(state.receive, receive);

  let vault_account =
      get_spl_account::<SplAccount>(&svm, &vault).unwrap();

  assert_eq!(vault_account.amount, amount);

  let maker_account =
      get_spl_account::<SplAccount>(&svm, &maker_token_a).unwrap();

  assert_eq!(maker_account.amount, 500_000_000);

  println!("Escrow make test passed!");
}

#[test]
fn test_take_escrow() {
  let program_id = escrow::id();

  let maker = Keypair::new();
  let taker = Keypair::new();

  let maker_pubkey = maker.pubkey();
  let taker_pubkey = taker.pubkey();

  let (escrow_pda, _) = Pubkey::find_program_address(
      &[b"escrow", maker_pubkey.as_ref()],
      &program_id,
  );

  let mut svm = setup_svm();
  load_escrow_program(&mut svm);

  svm.airdrop(&maker_pubkey, 5_000_000_000).unwrap();
  svm.airdrop(&taker_pubkey, 5_000_000_000).unwrap();

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

  // Create maker's Token A account.
  let maker_token_a =
      CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
          .send()
          .unwrap();

  // Create maker's Token B account.
  let maker_token_b =
      CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
          .send()
          .unwrap();

  // Create taker's Token A account.
  let taker_token_a =
      CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
          .send()
          .unwrap();

  // Create taker's Token B account.
  let taker_token_b =
      CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
          .send()
          .unwrap();

  // Give maker 1,000 Token A.
  MintTo::new(
      &mut svm,
      &maker,
      &mint_a,
      &maker_token_a,
      1_000_000_000,
  )
  .send()
  .unwrap();

  // Give taker 250 Token B.
  MintTo::new(
      &mut svm,
      &maker,
      &mint_b,
      &taker_token_b,
      250_000_000,
  )
  .send()
  .unwrap();

  let vault =
      associated_token::get_associated_token_address(&escrow_pda, &mint_a);

  let amount = 500_000_000u64;
  let receive = 250_000_000u64;

  // STEP 1: Maker creates the escrow and deposits Token A.
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

  let make_message = Message::new_with_blockhash(
      &[make_ix],
      Some(&maker_pubkey),
      &svm.latest_blockhash(),
  );

  let make_transaction = VersionedTransaction::try_new(
      VersionedMessage::Legacy(make_message),
      &[&maker],
  )
  .unwrap();

  svm.send_transaction(make_transaction)
      .expect("Make instruction failed");

  // STEP 2: Taker accepts the offer.
  let take_ix = Instruction::new_with_bytes(
      program_id,
      &escrow::instruction::Take {}.data(),
      escrow::accounts::Take {
          taker: taker_pubkey,
          maker: maker_pubkey,
          escrow: escrow_pda,
          mint_a,
          mint_b,
          taker_token_b,
          maker_token_b,
          vault,
          taker_token_a,
          token_program: anchor_spl::token::ID,
      }
      .to_account_metas(None),
  );

  let take_message = Message::new_with_blockhash(
      &[take_ix],
      Some(&taker_pubkey),
      &svm.latest_blockhash(),
  );

  let take_transaction = VersionedTransaction::try_new(
      VersionedMessage::Legacy(take_message),
      &[&taker],
  )
  .unwrap();

  svm.send_transaction(take_transaction)
      .expect("Take instruction failed");

  // STEP 3: Verify taker's Token A balance.
  let taker_a =
      get_spl_account::<SplAccount>(&svm, &taker_token_a).unwrap();

  assert_eq!(taker_a.amount, 500_000_000);

  // STEP 4: Verify maker received Token B.
  let maker_b =
      get_spl_account::<SplAccount>(&svm, &maker_token_b).unwrap();

  assert_eq!(maker_b.amount, 250_000_000);

  // STEP 5: Verify taker's Token B was spent.
  let taker_b =
      get_spl_account::<SplAccount>(&svm, &taker_token_b).unwrap();

  assert_eq!(taker_b.amount, 0);

  // STEP 6: Verify maker's remaining Token A.
  let maker_a =
      get_spl_account::<SplAccount>(&svm, &maker_token_a).unwrap();

  assert_eq!(maker_a.amount, 500_000_000);

  // STEP 7: Verify vault and escrow state were closed.
  assert!(
      svm.get_account(&vault).is_none(),
      "Vault should be closed"
  );

  assert!(
      svm.get_account(&escrow_pda).is_none(),
      "Escrow state should be closed"
  );

  println!("Escrow take test passed!");
}


#[test]
fn test_refund_escrow() {
    let program_id = escrow::id();

    let maker = Keypair::new();
    let maker_pubkey = maker.pubkey();

    let (escrow_pda, _) = Pubkey::find_program_address(
        &[b"escrow", maker_pubkey.as_ref()],
        &program_id,
    );

    let mut svm = setup_svm();
    load_escrow_program(&mut svm);

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

    // Create maker's Token A account.
    let maker_token_a =
        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
            .send()
            .unwrap();

    // Give maker 1,000 Token A.
    MintTo::new(
        &mut svm,
        &maker,
        &mint_a,
        &maker_token_a,
        1_000_000_000,
    )
    .send()
    .unwrap();

    let vault =
        associated_token::get_associated_token_address(&escrow_pda, &mint_a);

    let amount = 500_000_000u64;
    let receive = 250_000_000u64;

    // STEP 1: Maker creates an escrow offer.
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

    let make_message = Message::new_with_blockhash(
        &[make_ix],
        Some(&maker_pubkey),
        &svm.latest_blockhash(),
    );

    let make_transaction = VersionedTransaction::try_new(
        VersionedMessage::Legacy(make_message),
        &[&maker],
    )
    .unwrap();

    svm.send_transaction(make_transaction)
        .expect("Make instruction failed");

    // STEP 2: Maker cancels the offer using refund.
    let refund_ix = Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Refund {}.data(),
        escrow::accounts::Refund {
            maker: maker_pubkey,
            escrow: escrow_pda,
            mint_a,
            vault,
            maker_token_a,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
    );

    let refund_message = Message::new_with_blockhash(
        &[refund_ix],
        Some(&maker_pubkey),
        &svm.latest_blockhash(),
    );

    let refund_transaction = VersionedTransaction::try_new(
        VersionedMessage::Legacy(refund_message),
        &[&maker],
    )
    .unwrap();

    svm.send_transaction(refund_transaction)
        .expect("Refund instruction failed");

    // STEP 3: Verify maker received all deposited Token A back.
    let maker_a =
        get_spl_account::<SplAccount>(&svm, &maker_token_a).unwrap();

    assert_eq!(maker_a.amount, 1_000_000_000);

    // STEP 4: Verify vault and escrow state were closed.
    assert!(
        svm.get_account(&vault).is_none(),
        "Vault should be closed"
    );

    assert!(
        svm.get_account(&escrow_pda).is_none(),
        "Escrow state should be closed"
    );

    println!("Escrow refund test passed!");
  }



#[test]
fn test_make_rejects_identical_mints() {
    let program_id = escrow::id();

    let maker = Keypair::new();
    let maker_pubkey = maker.pubkey();

    let (escrow_pda, _) = Pubkey::find_program_address(
        &[b"escrow", maker_pubkey.as_ref()],
        &program_id,
    );

    let mut svm = setup_svm();
    load_escrow_program(&mut svm);

    svm.airdrop(&maker_pubkey, 5_000_000_000).unwrap();

    // Create one mint and use it as both Token A and Token B.
    let mint = CreateMint::new(&mut svm, &maker)
        .authority(&maker_pubkey)
        .decimals(6)
        .send()
        .unwrap();

    let maker_token_a =
        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .send()
            .unwrap();

    MintTo::new(
        &mut svm,
        &maker,
        &mint,
        &maker_token_a,
        1_000_000_000,
    )
    .send()
    .unwrap();

    let vault =
        associated_token::get_associated_token_address(&escrow_pda, &mint);

    let make_ix = Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Make {
            amount: 500_000_000,
            receive: 250_000_000,
        }
        .data(),
        escrow::accounts::Make {
            maker: maker_pubkey,
            escrow: escrow_pda,
            mint_a: mint,
            mint_b: mint,
            maker_token_a,
            vault,
            token_program: anchor_spl::token::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let message = Message::new_with_blockhash(
        &[make_ix],
        Some(&maker_pubkey),
        &svm.latest_blockhash(),
    );

    let transaction = VersionedTransaction::try_new(
        VersionedMessage::Legacy(message),
        &[&maker],
    )
    .unwrap();

    let result = svm.send_transaction(transaction);

    assert!(
        result.is_err(),
        "Make should reject identical Token A and Token B mints"
    );

    // Failed transaction must not create the escrow or vault.
    assert!(svm.get_account(&escrow_pda).is_none());
    assert!(svm.get_account(&vault).is_none());

    println!("Identical mint rejection test passed!");
}
