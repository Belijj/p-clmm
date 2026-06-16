
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub use super::swap::SwapArgs;

#[repr(usize)]
#[allow(dead_code)]
pub enum AccountIdx {
    Payer = 0,
    AmmConfig = 1,
    PoolState = 2,
    InputTokenAccount = 3,
    OutputTokenAccount = 4,
    InputVault = 5,
    OutputVault = 6,
    ObservationState = 7,
    TokenProgram = 8,
    TokenProgram2022 = 9,
    MemoProgram = 10,
    InputVaultMint = 11,
    OutputVaultMint = 12,
}

const MIN_ACCOUNTS: usize = AccountIdx::OutputVaultMint as usize + 1;

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < MIN_ACCOUNTS {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let _args = SwapArgs::try_from_bytes(data)?;
    let _ = (program_id, accounts);
    Err(ProgramError::InvalidInstructionData)
}
