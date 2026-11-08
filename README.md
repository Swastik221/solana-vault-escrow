# Solana Vault & Token Escrow

A collection of two Solana programs built using **Rust, Anchor Framework, and LiteSVM**.

This project demonstrates secure SOL custody through Program Derived Addresses (PDAs), SPL Token transfers through Cross-Program Invocations (CPIs), and trust-minimized token exchange using an escrow mechanism.

## Programs

| Program | Description |
|---|---|
| **SOL Vault** | Deposit, withdraw, and securely close a SOL vault. |
| **SPL Token Escrow** | Create token exchange offers, accept offers, or refund deposited tokens. |

## Tech Stack

- Rust
- Solana
- Anchor Framework
- Anchor SPL
- LiteSVM
- SPL Token Program
- Associated Token Account Program

## Project Structure

```text
solana-vault-escrow/
├── Anchor.toml
├── Cargo.toml
├── programs/
│   ├── vault/
│   │   ├── src/
│   │   │   ├── instructions/
│   │   │   │   ├── initialize.rs
│   │   │   │   ├── deposit.rs
│   │   │   │   ├── withdraw.rs
│   │   │   │   └── close.rs
│   │   │   ├── constants.rs
│   │   │   ├── error.rs
│   │   │   ├── state.rs
│   │   │   └── lib.rs
│   │   └── tests/
│   │       └── test_vault.rs
│   │
│   └── escrow/
│       ├── src/
│       │   ├── instructions/
│       │   │   ├── make.rs
│       │   │   ├── take.rs
│       │   │   └── refund.rs
│       │   ├── constants.rs
│       │   ├── error.rs
│       │   ├── state.rs
│       │   └── lib.rs
│       └── tests/
│           └── test_escrow.rs
│
└── README.md
```

## 1. SOL Vault Program

The Vault program allows a user to manage SOL through a PDA-controlled vault.

### Program ID

```text
EQANduK3KtQnBCN6tnGvbR2hARLyrkcV7kNGsTPX3gnE
```

### PDA Architecture

Two PDAs are used:

**Vault State PDA**

```rust
seeds = [b"state", user.key().as_ref()]
```

Stores:
- User public key
- Vault PDA bump
- State PDA bump

**Vault PDA**

```rust
seeds = [b"vault", vault_state.key().as_ref()]
```

A System Program-owned PDA that holds SOL.

### Instructions

| Instruction | Functionality |
|---|---|
| `initialize` | Creates the Vault State PDA and SOL vault PDA. |
| `deposit` | Transfers SOL from the user into the vault. |
| `withdraw` | Withdraws SOL using PDA signing while preserving the vault's rent-exempt minimum. |
| `close` | Returns the entire vault balance and closes the state account. |

### Important Implementation Details

- PDA derivation and bump validation.
- System Program CPIs for SOL transfers.
- PDA signing through `CpiContext::new_with_signer`.
- Ownership validation using `has_one`.
- Positive deposit and withdrawal amount validation.
- Rent-exemption protection during withdrawals.
- Account closure and rent recovery.

## 2. SPL Token Escrow Program

The Escrow program enables two users to exchange SPL tokens without requiring trust in either party.

A maker deposits Token A into an escrow-controlled associated token account and specifies how much Token B they expect in return.

The taker can accept the offer, or the maker can cancel it and recover the deposited tokens.

### Program ID

```text
T89b167ssQxwDN8W4o19Xk3mjdDSWoMpMemL3HjHNm5
```

### Escrow State

The Escrow account stores:

| Field | Type | Purpose |
|---|---|---|
| `maker` | `Pubkey` | Original offer creator |
| `mint_a` | `Pubkey` | Token deposited by maker |
| `mint_b` | `Pubkey` | Token expected from taker |
| `receive` | `u64` | Required Token B amount |
| `bump` | `u8` | Escrow PDA bump |

### PDA Architecture

**Escrow PDA**

```rust
seeds = [b"escrow", maker.key().as_ref()]
```

The escrow PDA is the authority over the vault's associated token account.

**Vault Associated Token Account**

Derived using:

- Owner: Escrow PDA
- Mint: Token A mint

The vault securely holds the maker's deposited tokens until the offer is completed or refunded.

### Instructions

**1. `make(amount, receive)`**

The maker creates an offer.

- Initializes the Escrow PDA.
- Creates the escrow's Token A associated token account.
- Stores the exchange terms.
- Transfers Token A from the maker into the vault.
- Rejects zero amounts and identical token mints.

**2. `take()`**

A taker accepts an existing offer.

- Transfers the required Token B from taker to maker.
- Uses the escrow PDA as signer to transfer Token A to the taker.
- Closes the empty vault.
- Closes the escrow state account.
- Returns account rent to the maker.

**3. `refund()`**

The maker cancels an existing offer.

- Requires the original maker's signature.
- Transfers all deposited Token A back to the maker.
- Closes the empty vault.
- Closes the escrow state account.
- Returns account rent to the maker.

### Escrow Flow

```text
             MAKER
               |
               | make(amount, receive)
               |
               v
       ESCROW PDA + VAULT
               |
               |
        Offer is active
               |
          +----+----+
          |         |
          v         v
        TAKE      REFUND
          |         |
          v         v
      Taker gets  Maker gets
      Token A     Token A back
          |         |
          +----+----+
               |
         Accounts closed
```

### Important Implementation Details

- PDA-based escrow authority.
- SPL Token Program CPIs.
- Associated Token Account creation.
- PDA signer seeds for token transfers.
- Anchor account constraints and ownership validation.
- Automatic escrow and vault closure.
- Transaction atomicity: failed instructions do not partially execute.

## 3. Testing

The programs use **LiteSVM** for local Solana program execution and integration testing.

### Run all workspace builds and tests

```bash
anchor build
```

### Run Vault tests

```bash
cargo test -p vault --test test_vault -- --nocapture
```

### Run Escrow tests

```bash
cargo test -p escrow --test test_escrow -- --nocapture
```

### Escrow test coverage

| Test | Expected behavior |
|---|---|
| `test_make_escrow` | Creates offer and deposits Token A. |
| `test_take_escrow` | Exchanges Token A and Token B successfully. |
| `test_refund_escrow` | Returns deposited Token A to maker. |
| `test_make_rejects_identical_mints` | Rejects offers using the same mint for both tokens. |

The current Escrow integration test suite passes **4/4 tests**.

The Vault integration test suite has also been implemented and tested.

## 4. Running Locally

### Prerequisites

Install the following:

- Rust
- Solana CLI
- Anchor Framework
- Node.js and Yarn (for Anchor workspace tooling)

### Clone the repository

```bash
git clone <YOUR_GITHUB_REPOSITORY_URL>
cd solana-vault-escrow
```

### Build

```bash
anchor build
```

### Test

```bash
cargo test
```

## 5. Learning Outcomes

Through this project, I explored and implemented:

- Solana account model and ownership.
- Program Derived Addresses (PDAs).
- PDA signer seeds and bump derivation.
- SOL transfers using the System Program.
- SPL Token accounts and associated token accounts.
- Cross-Program Invocations (CPIs).
- Anchor account constraints.
- Rent-exemption and account closure.
- Rust-based Solana integration testing using LiteSVM.

## 6. Future Improvements

- Add dedicated unauthorized-access and adversarial tests.
- Add escrow expiration and timeout mechanisms.
- Support multiple simultaneous escrow offers per maker.
- Add client-side TypeScript integration.
- Add devnet deployment and transaction demonstrations.

## License

Add your preferred open-source license before publishing this repository.
