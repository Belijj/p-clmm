#![no_std]

#[cfg(test)]
extern crate std;

pub mod error;
pub mod instructions;
pub mod libraries;
pub mod states;
pub mod util;

pub use instructions::swap_internal::{swap_internal, SwapInternalResult};

pub mod discriminator {
    include!(concat!(env!("OUT_DIR"), "/anchor_discriminators.rs"));
}

pub use core as core_;

pub type Result<T> = core::result::Result<T, pinocchio::program_error::ProgramError>;

#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr $(,)?) => {
        if !($cond) {
            return ::core::result::Result::Err($err.into());
        }
    };
}

#[macro_export]
macro_rules! require_gt {
    ($a:expr, $b:expr $(,)?) => {
        if !($a > $b) {
            return ::core::result::Result::Err(
                ::pinocchio::program_error::ProgramError::InvalidArgument,
            );
        }
    };
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if !($a > $b) {
            return ::core::result::Result::Err($err.into());
        }
    };
}

#[macro_export]
macro_rules! require_gte {
    ($a:expr, $b:expr $(,)?) => {
        if !($a >= $b) {
            return ::core::result::Result::Err(
                ::pinocchio::program_error::ProgramError::InvalidArgument,
            );
        }
    };
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if !($a >= $b) {
            return ::core::result::Result::Err($err.into());
        }
    };
}

#[macro_export]
macro_rules! require_eq {
    ($a:expr, $b:expr $(,)?) => {
        if $a != $b {
            return ::core::result::Result::Err(
                ::pinocchio::program_error::ProgramError::InvalidArgument,
            );
        }
    };
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a != $b {
            return ::core::result::Result::Err($err.into());
        }
    };
}

#[macro_export]
macro_rules! require_neq {
    ($a:expr, $b:expr $(,)?) => {
        if $a == $b {
            return ::core::result::Result::Err(
                ::pinocchio::program_error::ProgramError::InvalidArgument,
            );
        }
    };
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a == $b {
            return ::core::result::Result::Err($err.into());
        }
    };
}

#[macro_export]
macro_rules! err {
    ($err:expr $(,)?) => {
        ::core::result::Result::Err($err.into())
    };
}

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

#[cfg(feature = "devnet")]
pinocchio_pubkey::declare_id!("DRayAUgENGQBKVaX8owNhgzkEDyoHTGVEGHVJT1E9pfH");
#[cfg(not(feature = "devnet"))]
pinocchio_pubkey::declare_id!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (discriminator, data) = instruction_data.split_at(8);
    let disc: [u8; 8] = discriminator.try_into().unwrap();

    match disc {
        discriminator::instruction::SWAP => instructions::swap::process(program_id, accounts, data),
        discriminator::instruction::SWAP_V2 => {
            instructions::swap_v2::process(program_id, accounts, data)
        }
        discriminator::instruction::CREATE_POOL => {
            instructions::create_pool::process(program_id, accounts, data)
        }
        discriminator::instruction::INCREASE_LIQUIDITY_V2 => {
            instructions::increase_liquidity::process(program_id, accounts, data)
        }
        discriminator::instruction::DECREASE_LIQUIDITY_V2 => {
            instructions::decrease_liquidity::process(program_id, accounts, data)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
