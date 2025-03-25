#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Anchor ⚓ is a framework for Solana's Sealevel runtime providing several
//! convenient developer tools.
//!
//! - Rust eDSL for writing safe, secure, and high level Solana programs
//! - [IDL](https://en.wikipedia.org/wiki/Interface_description_language) specification
//! - TypeScript package for generating clients from IDL
//! - CLI and workspace management for developing complete applications
//!
//! If you're familiar with developing in Ethereum's
//! [Solidity](https://docs.soliditylang.org/en/v0.7.4/),
//! [Truffle](https://www.trufflesuite.com/),
//! [web3.js](https://github.com/ethereum/web3.js) or Parity's
//! [Ink!](https://github.com/paritytech/ink), then the experience will be
//! familiar. Although the syntax and semantics are targeted at Solana, the high
//! level workflow of writing RPC request handlers, emitting an IDL, and
//! generating clients from IDL is the same.
//!
//! For detailed tutorials and examples on how to use Anchor, see the guided
//! [tutorials](https://anchor-lang.com) or examples in the GitHub
//! [repository](https://github.com/coral-xyz/anchor).
//!
//! Presented here are the Rust primitives for building on Solana.

extern crate self as anchor_lang;

use crate::solana_program::account_info::AccountInfo;
use crate::solana_program::instruction::AccountMeta;
use crate::solana_program::program_error::ProgramError;
use crate::solana_program::pubkey::Pubkey;
use bytemuck::{Pod, Zeroable};
use std::{collections::BTreeSet, fmt::Debug, io::Write};

mod account_meta;
pub mod accounts;
mod bpf_upgradeable_state;
mod bpf_writer;
mod common;
pub mod context;
pub mod error;
#[doc(hidden)]
pub mod event;
#[doc(hidden)]
pub mod idl;
pub mod system_program;
mod vec;

#[cfg(feature = "lazy-account")]
mod lazy;

pub use crate::bpf_upgradeable_state::*;
pub use anchor_attribute_access_control::access_control;
pub use anchor_attribute_account::{account, declare_id, pubkey, zero_copy};
pub use anchor_attribute_constant::constant;
pub use anchor_attribute_error::*;
pub use anchor_attribute_event::{emit, event};
pub use anchor_attribute_program::{declare_program, instruction, program};
pub use anchor_derive_accounts::Accounts;
pub use anchor_derive_serde::{AnchorDeserialize, AnchorSerialize};
pub use anchor_derive_space::InitSpace;

/// Borsh is the default serialization format for instructions and accounts.
pub use borsh::de::BorshDeserialize as AnchorDeserialize;
pub use borsh::ser::BorshSerialize as AnchorSerialize;
pub mod solana_program {
    pub use {
        solana_account_info as account_info, solana_clock as clock, solana_cpi as program,
        solana_instruction as instruction, solana_msg::msg, solana_program_error as program_error,
        solana_program_memory as program_memory, solana_pubkey as pubkey,
        solana_sdk_ids::system_program, solana_system_interface::instruction as system_instruction,
    };
    pub mod bpf_loader_upgradeable {
        #[allow(deprecated)]
        pub use solana_loader_v3_interface::{
            get_program_data_address,
            instruction::{
                close, close_any, create_buffer, deploy_with_max_program_len, extend_program,
                is_close_instruction, is_set_authority_checked_instruction,
                is_set_authority_instruction, is_upgrade_instruction, set_buffer_authority,
                set_buffer_authority_checked, set_upgrade_authority, set_upgrade_authority_checked,
                upgrade, write,
            },
            state::UpgradeableLoaderState,
        };
        pub use solana_sdk_ids::bpf_loader_upgradeable::{check_id, id, ID};
    }

    pub mod log {
        pub use solana_msg::{msg, sol_log};
    }
    pub mod sysvar {
        pub use solana_sysvar_id::{declare_deprecated_sysvar_id, declare_sysvar_id, SysvarId};
        #[deprecated(since = "2.2.0", note = "Use `solana-sysvar` crate instead")]
        #[allow(deprecated)]
        pub use {
            solana_sdk_ids::sysvar::{check_id, id, ID},
            solana_sysvar::{
                clock, epoch_rewards, epoch_schedule, fees, is_sysvar_id, last_restart_slot,
                recent_blockhashes, rent, rewards, slot_hashes, slot_history, stake_history,
                Sysvar, ALL_IDS,
            },
        };
        pub mod instructions {
            pub use solana_instruction::{BorrowedAccountMeta, BorrowedInstruction};
            #[cfg(not(target_os = "solana"))]
            pub use solana_instructions_sysvar::construct_instructions_data;
            #[deprecated(
                since = "2.2.0",
                note = "Use solana-instructions-sysvar crate instead"
            )]
            pub use solana_instructions_sysvar::{
                get_instruction_relative, load_current_index_checked, load_instruction_at_checked,
                store_current_index, Instructions,
            };
            #[deprecated(since = "2.2.0", note = "Use solana-sdk-ids crate instead")]
            pub use solana_sdk_ids::sysvar::instructions::{check_id, id, ID};
        }
    }
}

#[cfg(feature = "event-cpi")]
pub use anchor_attribute_event::{emit_cpi, event_cpi};

#[cfg(feature = "idl-build")]
pub use idl::IdlBuild;

#[cfg(feature = "interface-instructions")]
pub use anchor_attribute_program::interface;

pub type Result<T> = std::result::Result<T, error::Error>;

/// A data structure of validated accounts that can be deserialized from the
/// input to a Solana program. Implementations of this trait should perform any
/// and all requisite constraint checks on accounts to ensure the accounts
/// maintain any invariants required for the program to run securely. In most
/// cases, it's recommended to use the [`Accounts`](./derive.Accounts.html)
/// derive macro to implement this trait.
///
/// Generics:
/// -   `B`: the type of the PDA bumps cache struct generated by the `Accounts` struct.
///     For example,
/// ```rust,ignore
/// pub struct Example<'info> {
///     #[account(
///         init,
///         seeds = [...],
///         bump,
///     )]
///     pub pda_1: UncheckedAccount<'info>,
///     pub not_pda: UncheckedAccount<'info>,
/// }
/// ```
///
///    generates:
///
/// ```rust,ignore
/// pub struct ExampleBumps {
///     pub pda_1: u8,
/// }
/// ```
pub trait Accounts<'info, B>: ToAccountMetas + ToAccountInfos<'info> + Sized {
    /// Returns the validated accounts struct. What constitutes "valid" is
    /// program dependent. However, users of these types should never have to
    /// worry about account substitution attacks. For example, if a program
    /// expects a `Mint` account from the SPL token program  in a particular
    /// field, then it should be impossible for this method to return `Ok` if
    /// any other account type is given--from the SPL token program or elsewhere.
    ///
    /// `program_id` is the currently executing program. `accounts` is the
    /// set of accounts to construct the type from. For every account used,
    /// the implementation should mutate the slice, consuming the used entry
    /// so that it cannot be used again.
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo<'info>],
        ix_data: &[u8],
        bumps: &mut B,
        reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self>;
}

/// Associated bump seeds for `Accounts`.
pub trait Bumps {
    /// Struct to hold account bump seeds.
    type Bumps: Sized + Debug;
}

/// The exit procedure for an account. Any cleanup or persistence to storage
/// should be done here.
pub trait AccountsExit<'info>: ToAccountMetas + ToAccountInfos<'info> {
    /// `program_id` is the currently executing program.
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        // no-op
        Ok(())
    }
}

/// The close procedure to initiate garabage collection of an account, allowing
/// one to retrieve the rent exemption.
pub trait AccountsClose<'info>: ToAccountInfos<'info> {
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()>;
}

/// Transformation to
/// [`AccountMeta`](../solana_program/instruction/struct.AccountMeta.html)
/// structs.
pub trait ToAccountMetas {
    /// `is_signer` is given as an optional override for the signer meta field.
    /// This covers the edge case when a program-derived-address needs to relay
    /// a transaction from a client to another program but sign the transaction
    /// before the relay. The client cannot mark the field as a signer, and so
    /// we have to override the is_signer meta field given by the client.
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta>;
}

/// Transformation to
/// [`AccountInfo`](../solana_program/account_info/struct.AccountInfo.html)
/// structs.
pub trait ToAccountInfos<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;
}

/// Transformation to an `AccountInfo` struct.
pub trait ToAccountInfo<'info> {
    fn to_account_info(&self) -> AccountInfo<'info>;
}

impl<'info, T> ToAccountInfo<'info> for T
where
    T: AsRef<AccountInfo<'info>>,
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.as_ref().clone()
    }
}

/// Lamports related utility methods for accounts.
pub trait Lamports<'info>: AsRef<AccountInfo<'info>> {
    /// Get the lamports of the account.
    fn get_lamports(&self) -> u64 {
        self.as_ref().lamports()
    }

    /// Add lamports to the account.
    ///
    /// This method is useful for transferring lamports from a PDA.
    ///
    /// # Requirements
    ///
    /// 1. The account must be marked `mut`.
    /// 2. The total lamports **before** the transaction must equal to total lamports **after**
    ///    the transaction.
    /// 3. `lamports` field of the account info should not currently be borrowed.
    ///
    /// See [`Lamports::sub_lamports`] for subtracting lamports.
    fn add_lamports(&self, amount: u64) -> Result<&Self> {
        **self.as_ref().try_borrow_mut_lamports()? = self
            .get_lamports()
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(self)
    }

    /// Subtract lamports from the account.
    ///
    /// This method is useful for transferring lamports from a PDA.
    ///
    /// # Requirements
    ///
    /// 1. The account must be owned by the executing program.
    /// 2. The account must be marked `mut`.
    /// 3. The total lamports **before** the transaction must equal to total lamports **after**
    ///    the transaction.
    /// 4. `lamports` field of the account info should not currently be borrowed.
    ///
    /// See [`Lamports::add_lamports`] for adding lamports.
    fn sub_lamports(&self, amount: u64) -> Result<&Self> {
        **self.as_ref().try_borrow_mut_lamports()? = self
            .get_lamports()
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(self)
    }
}

impl<'info, T: AsRef<AccountInfo<'info>>> Lamports<'info> for T {}

/// A data structure that can be serialized and stored into account storage,
/// i.e. an
/// [`AccountInfo`](../solana_program/account_info/struct.AccountInfo.html#structfield.data)'s
/// mutable data slice.
///
/// Implementors of this trait should ensure that any subsequent usage of the
/// `AccountDeserialize` trait succeeds if and only if the account is of the
/// correct type.
///
/// In most cases, one can use the default implementation provided by the
/// [`#[account]`](./attr.account.html) attribute.
pub trait AccountSerialize {
    /// Serializes the account data into `writer`.
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> Result<()> {
        Ok(())
    }
}

/// A data structure that can be deserialized and stored into account storage,
/// i.e. an
/// [`AccountInfo`](../solana_program/account_info/struct.AccountInfo.html#structfield.data)'s
/// mutable data slice.
pub trait AccountDeserialize: Sized {
    /// Deserializes previously initialized account data. Should fail for all
    /// uninitialized accounts, where the bytes are zeroed. Implementations
    /// should be unique to a particular account type so that one can never
    /// successfully deserialize the data of one account type into another.
    /// For example, if the SPL token program were to implement this trait,
    /// it should be impossible to deserialize a `Mint` account into a token
    /// `Account`.
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }

    /// Deserializes account data without checking the account discriminator.
    /// This should only be used on account initialization, when the bytes of
    /// the account are zeroed.
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self>;
}

/// An account data structure capable of zero copy deserialization.
pub trait ZeroCopy: Discriminator + Copy + Clone + Zeroable + Pod {}

/// Calculates the data for an instruction invocation, where the data is
/// `Discriminator + BorshSerialize(args)`. `args` is a borsh serialized
/// struct of named fields for each argument given to an instruction.
pub trait InstructionData: Discriminator + AnchorSerialize {
    fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }

    /// Clears `data` and writes instruction data to it.
    ///
    /// We use a `Vec<u8>`` here because of the additional flexibility of re-allocation (only if
    /// necessary), and because the data field in `Instruction` expects a `Vec<u8>`.
    fn write_to(&self, mut data: &mut Vec<u8>) {
        data.clear();
        data.extend_from_slice(Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap()
    }
}

/// An event that can be emitted via a Solana log. See [`emit!`](crate::prelude::emit) for an example.
pub trait Event: AnchorSerialize + AnchorDeserialize + Discriminator {
    fn data(&self) -> Vec<u8>;
}

/// Unique identifier for a type.
///
/// This is not a trait you should derive manually, as various Anchor macros already derive it
/// internally.
///
/// Prior to Anchor v0.31, discriminators were always 8 bytes in size. However, starting with Anchor
/// v0.31, it is possible to override the default discriminators, and discriminator length is no
/// longer fixed, which means this trait can also be implemented for non-Anchor programs.
///
/// It's important that the discriminator is always unique for the type you're implementing it
/// for. While the discriminator can be at any length (including zero), the IDL generation does not
/// currently allow empty discriminators for safety and convenience reasons. However, the trait
/// definition still allows empty discriminators because some non-Anchor programs, e.g. the SPL
/// Token program, don't have account discriminators. In that case, safety checks should never
/// depend on the discriminator.
pub trait Discriminator {
    /// Discriminator slice.
    ///
    /// See [`Discriminator`] trait documentation for more information.
    const DISCRIMINATOR: &'static [u8];
}

/// Defines the space of an account for initialization.
pub trait Space {
    const INIT_SPACE: usize;
}

/// Bump seed for program derived addresses.
pub trait Bump {
    fn seed(&self) -> u8;
}

/// Defines an address expected to own an account.
pub trait Owner {
    fn owner() -> Pubkey;
}

/// Defines a list of addresses expected to own an account.
pub trait Owners {
    fn owners() -> &'static [Pubkey];
}

/// Defines a trait for checking the owner of a program.
pub trait CheckOwner {
    fn check_owner(owner: &Pubkey) -> Result<()>;
}

impl<T: Owners> CheckOwner for T {
    fn check_owner(owner: &Pubkey) -> Result<()> {
        if !Self::owners().contains(owner) {
            Err(
                error::Error::from(error::ErrorCode::AccountOwnedByWrongProgram)
                    .with_account_name(*owner),
            )
        } else {
            Ok(())
        }
    }
}

/// Defines the id of a program.
pub trait Id {
    fn id() -> Pubkey;
}

/// Defines the possible ids of a program.
pub trait Ids {
    fn ids() -> &'static [Pubkey];
}

/// Defines a trait for checking the id of a program.
pub trait CheckId {
    fn check_id(id: &Pubkey) -> Result<()>;
}

impl<T: Ids> CheckId for T {
    fn check_id(id: &Pubkey) -> Result<()> {
        if !Self::ids().contains(id) {
            Err(error::Error::from(error::ErrorCode::InvalidProgramId).with_account_name(*id))
        } else {
            Ok(())
        }
    }
}

/// Defines the Pubkey of an account.
pub trait Key {
    fn key(&self) -> Pubkey;
}

impl Key for Pubkey {
    fn key(&self) -> Pubkey {
        *self
    }
}

/// The prelude contains all commonly used components of the crate.
/// All programs should include it via `anchor_lang::prelude::*;`.
pub mod prelude {
    pub use super::{
        access_control, account, accounts::account::Account,
        accounts::account_loader::AccountLoader, accounts::interface::Interface,
        accounts::interface_account::InterfaceAccount, accounts::program::Program,
        accounts::signer::Signer, accounts::system_account::SystemAccount,
        accounts::sysvar::Sysvar, accounts::unchecked_account::UncheckedAccount, constant,
        context::Context, context::CpiContext, declare_id, declare_program, emit, err, error,
        event, instruction, program, pubkey, require, require_eq, require_gt, require_gte,
        require_keys_eq, require_keys_neq, require_neq,
        solana_program::bpf_loader_upgradeable::UpgradeableLoaderState, source,
        system_program::System, zero_copy, AccountDeserialize, AccountSerialize, Accounts,
        AccountsClose, AccountsExit, AnchorDeserialize, AnchorSerialize, Discriminator, Id,
        InitSpace, Key, Lamports, Owner, ProgramData, Result, Space, ToAccountInfo, ToAccountInfos,
        ToAccountMetas,
    };
    pub use crate::solana_program::account_info::{next_account_info, AccountInfo};
    pub use crate::solana_program::instruction::AccountMeta;
    pub use crate::solana_program::msg;
    pub use crate::solana_program::program_error::ProgramError;
    pub use crate::solana_program::pubkey::Pubkey;
    pub use crate::solana_program::sysvar::clock::Clock;
    pub use crate::solana_program::sysvar::epoch_schedule::EpochSchedule;
    pub use crate::solana_program::sysvar::instructions::Instructions;
    pub use crate::solana_program::sysvar::rent::Rent;
    pub use crate::solana_program::sysvar::rewards::Rewards;
    pub use crate::solana_program::sysvar::slot_hashes::SlotHashes;
    pub use crate::solana_program::sysvar::slot_history::SlotHistory;
    pub use crate::solana_program::sysvar::stake_history::StakeHistory;
    pub use crate::solana_program::sysvar::Sysvar as SolanaSysvar;
    pub use anchor_attribute_error::*;
    pub use borsh;
    pub use error::*;
    pub use thiserror;

    #[cfg(feature = "event-cpi")]
    pub use super::{emit_cpi, event_cpi};

    #[cfg(feature = "idl-build")]
    pub use super::idl::IdlBuild;

    #[cfg(feature = "interface-instructions")]
    pub use super::interface;

    #[cfg(feature = "lazy-account")]
    pub use super::accounts::lazy_account::LazyAccount;
}

/// Internal module used by macros and unstable apis.
#[doc(hidden)]
pub mod __private {
    pub use anchor_attribute_account::ZeroCopyAccessor;
    pub use base64;
    pub use bytemuck;

    pub use crate::{bpf_writer::BpfWriter, common::is_closed};

    use crate::solana_program::pubkey::Pubkey;

    // Used to calculate the maximum between two expressions.
    // It is necessary for the calculation of the enum space.
    #[doc(hidden)]
    pub const fn max(a: usize, b: usize) -> usize {
        [a, b][(a < b) as usize]
    }

    // Very experimental trait.
    #[doc(hidden)]
    pub trait ZeroCopyAccessor<Ty> {
        fn get(&self) -> Ty;
        fn set(input: &Ty) -> Self;
    }

    #[doc(hidden)]
    impl ZeroCopyAccessor<Pubkey> for [u8; 32] {
        fn get(&self) -> Pubkey {
            Pubkey::from(*self)
        }
        fn set(input: &Pubkey) -> [u8; 32] {
            input.to_bytes()
        }
    }

    #[cfg(feature = "lazy-account")]
    pub use crate::lazy::Lazy;
    #[cfg(feature = "lazy-account")]
    pub use anchor_derive_serde::Lazy;
}

/// Ensures a condition is true, otherwise returns with the given error.
/// Use this with or without a custom error type.
///
/// # Example
/// ```ignore
/// // Instruction function
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require!(ctx.accounts.data.mutation_allowed, MyError::MutationForbidden);
///     ctx.accounts.data.data = data;
///     Ok(())
/// }
///
/// // An enum for custom error codes
/// #[error_code]
/// pub enum MyError {
///     MutationForbidden
/// }
///
/// // An account definition
/// #[account]
/// #[derive(Default)]
/// pub struct MyData {
///     mutation_allowed: bool,
///     data: u64
/// }
///
/// // An account validation struct
/// #[derive(Accounts)]
/// pub struct SetData<'info> {
///     #[account(mut)]
///     pub data: Account<'info, MyData>
/// }
/// ```
#[macro_export]
macro_rules! require {
    ($invariant:expr, $error:tt $(,)?) => {
        if !($invariant) {
            return Err(anchor_lang::error!($crate::ErrorCode::$error));
        }
    };
    ($invariant:expr, $error:expr $(,)?) => {
        if !($invariant) {
            return Err(anchor_lang::error!($error));
        }
    };
}

/// Ensures two NON-PUBKEY values are equal.
///
/// Use [require_keys_eq](crate::prelude::require_keys_eq)
/// to compare two pubkeys.
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require_eq!(ctx.accounts.data.data, 0);
///     ctx.accounts.data.data = data;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! require_eq {
    ($value1: expr, $value2: expr, $error_code:expr $(,)?) => {
        if $value1 != $value2 {
            return Err(error!($error_code).with_values(($value1, $value2)));
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 != $value2 {
            return Err(error!(anchor_lang::error::ErrorCode::RequireEqViolated)
                .with_values(($value1, $value2)));
        }
    };
}

/// Ensures two NON-PUBKEY values are not equal.
///
/// Use [require_keys_neq](crate::prelude::require_keys_neq)
/// to compare two pubkeys.
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require_neq!(ctx.accounts.data.data, 0);
///     ctx.accounts.data.data = data;
///     Ok(());
/// }
/// ```
#[macro_export]
macro_rules! require_neq {
    ($value1: expr, $value2: expr, $error_code: expr $(,)?) => {
        if $value1 == $value2 {
            return Err(error!($error_code).with_values(($value1, $value2)));
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 == $value2 {
            return Err(error!(anchor_lang::error::ErrorCode::RequireNeqViolated)
                .with_values(($value1, $value2)));
        }
    };
}

/// Ensures two pubkeys values are equal.
///
/// Use [require_eq](crate::prelude::require_eq)
/// to compare two non-pubkey values.
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require_keys_eq!(ctx.accounts.data.authority.key(), ctx.accounts.authority.key());
///     ctx.accounts.data.data = data;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! require_keys_eq {
    ($value1: expr, $value2: expr, $error_code:expr $(,)?) => {
        if $value1 != $value2 {
            return Err(error!($error_code).with_pubkeys(($value1, $value2)));
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 != $value2 {
            return Err(error!(anchor_lang::error::ErrorCode::RequireKeysEqViolated)
                .with_pubkeys(($value1, $value2)));
        }
    };
}

/// Ensures two pubkeys are not equal.
///
/// Use [require_neq](crate::prelude::require_neq)
/// to compare two non-pubkey values.
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require_keys_neq!(ctx.accounts.data.authority.key(), ctx.accounts.other.key());
///     ctx.accounts.data.data = data;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! require_keys_neq {
    ($value1: expr, $value2: expr, $error_code: expr $(,)?) => {
        if $value1 == $value2 {
            return Err(error!($error_code).with_pubkeys(($value1, $value2)));
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 == $value2 {
            return Err(
                error!(anchor_lang::error::ErrorCode::RequireKeysNeqViolated)
                    .with_pubkeys(($value1, $value2)),
            );
        }
    };
}

/// Ensures the first NON-PUBKEY value is greater than the second
/// NON-PUBKEY value.
///
/// To include an equality check, use [require_gte](crate::require_gte).
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require_gt!(ctx.accounts.data.data, 0);
///     ctx.accounts.data.data = data;
///     Ok(());
/// }
/// ```
#[macro_export]
macro_rules! require_gt {
    ($value1: expr, $value2: expr, $error_code: expr $(,)?) => {
        if $value1 <= $value2 {
            return Err(error!($error_code).with_values(($value1, $value2)));
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 <= $value2 {
            return Err(error!(anchor_lang::error::ErrorCode::RequireGtViolated)
                .with_values(($value1, $value2)));
        }
    };
}

/// Ensures the first NON-PUBKEY value is greater than or equal
/// to the second NON-PUBKEY value.
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
///     require_gte!(ctx.accounts.data.data, 1);
///     ctx.accounts.data.data = data;
///     Ok(());
/// }
/// ```
#[macro_export]
macro_rules! require_gte {
    ($value1: expr, $value2: expr, $error_code: expr $(,)?) => {
        if $value1 < $value2 {
            return Err(error!($error_code).with_values(($value1, $value2)));
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 < $value2 {
            return Err(error!(anchor_lang::error::ErrorCode::RequireGteViolated)
                .with_values(($value1, $value2)));
        }
    };
}

/// Returns with the given error.
/// Use this with a custom error type.
///
/// # Example
/// ```ignore
/// // Instruction function
/// pub fn example(ctx: Context<Example>) -> Result<()> {
///     err!(MyError::SomeError)
/// }
///
/// // An enum for custom error codes
/// #[error_code]
/// pub enum MyError {
///     SomeError
/// }
/// ```
#[macro_export]
macro_rules! err {
    ($error:tt $(,)?) => {
        Err(anchor_lang::error!($crate::ErrorCode::$error))
    };
    ($error:expr $(,)?) => {
        Err(anchor_lang::error!($error))
    };
}

/// Creates a [`Source`](crate::error::Source)
#[macro_export]
macro_rules! source {
    () => {
        anchor_lang::error::Source {
            filename: file!(),
            line: line!(),
        }
    };
}
