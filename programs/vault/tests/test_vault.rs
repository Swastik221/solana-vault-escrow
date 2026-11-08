use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_initialize_and_deposit() {
    let program_id = vault::id();
    let user = Keypair::new();

    // Derive the state PDA.
    let (vault_state, _) =
        Pubkey::find_program_address(&[b"state", user.pubkey().as_ref()], &program_id);

    // Derive the vault PDA.
    let (vault_account, _) =
        Pubkey::find_program_address(&[b"vault", vault_state.as_ref()], &program_id);

    // Start LiteSVM and load the compiled Vault program.
    let mut svm = LiteSVM::new();

    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/vault.so"));

    svm.add_program(program_id, bytes).unwrap();

    // Give the user 1 SOL.
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    // Initialize the vault.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &vault::instruction::Initialize {}.data(),
        vault::accounts::Initialize {
            user: user.pubkey(),
            vault_state,
            vault: vault_account,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[initialize_ix], Some(&user.pubkey()), &blockhash);

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&user]).unwrap();

    let result = svm.send_transaction(transaction);
    assert!(result.is_ok(), "Vault initialization failed");

    // Verify the stored VaultState.
    let state_account = svm.get_account(&vault_state).unwrap();

    let mut state_data: &[u8] = &state_account.data;

    let state = vault::state::VaultState::try_deserialize(&mut state_data).unwrap();

    assert_eq!(state.user, user.pubkey());

    // Record the vault balance before depositing.
    let balance_before = svm.get_account(&vault_account).unwrap().lamports;

    // Deposit 0.1 SOL = 100,000,000 lamports.
    let amount = 100_000_000u64;

    let deposit_ix = Instruction::new_with_bytes(
        program_id,
        &vault::instruction::Deposit { amount }.data(),
        vault::accounts::Deposit {
            user: user.pubkey(),
            vault_state,
            vault: vault_account,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[deposit_ix], Some(&user.pubkey()), &blockhash);

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&user]).unwrap();

    let result = svm.send_transaction(transaction);
    assert!(result.is_ok(), "Deposit failed");

    // Verify that the vault received exactly the deposited amount.
    let balance_after = svm.get_account(&vault_account).unwrap().lamports;

    assert_eq!(balance_after, balance_before + amount);

    println!("Vault initialization and deposit tests passed!");
}

#[test]
fn test_zero_amount_deposit_fails() {
    let program_id = vault::id();
    let user = Keypair::new();

    let (vault_state, _) =
        Pubkey::find_program_address(&[b"state", user.pubkey().as_ref()], &program_id);

    let (vault_account, _) =
        Pubkey::find_program_address(&[b"vault", vault_state.as_ref()], &program_id);

    let mut svm = LiteSVM::new();

    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/vault.so"));

    svm.add_program(program_id, bytes).unwrap();

    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    // Initialize first.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &vault::instruction::Initialize {}.data(),
        vault::accounts::Initialize {
            user: user.pubkey(),
            vault_state,
            vault: vault_account,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let message = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&user.pubkey()),
        &svm.latest_blockhash(),
    );

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&user]).unwrap();

    assert!(svm.send_transaction(transaction).is_ok());

    // Attempt a zero-lamport deposit.
    let deposit_ix = Instruction::new_with_bytes(
        program_id,
        &vault::instruction::Deposit { amount: 0 }.data(),
        vault::accounts::Deposit {
            user: user.pubkey(),
            vault_state,
            vault: vault_account,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let message =
        Message::new_with_blockhash(&[deposit_ix], Some(&user.pubkey()), &svm.latest_blockhash());

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&user]).unwrap();

    let result = svm.send_transaction(transaction);

    assert!(result.is_err(), "Zero amount deposit should fail");

    println!("Zero amount deposit correctly rejected!");
}
