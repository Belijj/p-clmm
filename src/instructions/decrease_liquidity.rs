
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub struct DecreaseLiquidityArgs {
    pub liquidity: u128,
    pub amount_0_min: u64,
    pub amount_1_min: u64,
}

impl DecreaseLiquidityArgs {
    pub const SIZE: usize = 16 + 8 + 8;

    #[inline(always)]
    pub fn try_from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::SIZE {
            return Err(ProgramError::InvalidInstructionData);
        }
        let p = data.as_ptr();
        let liquidity = u128::from_le_bytes(unsafe { *(p as *const [u8; 16]) });
        let amount_0_min = u64::from_le_bytes(unsafe { *(p.add(16) as *const [u8; 8]) });
        let amount_1_min = u64::from_le_bytes(unsafe { *(p.add(24) as *const [u8; 8]) });
        Ok(Self {
            liquidity,
            amount_0_min,
            amount_1_min,
        })
    }
}

#[repr(usize)]
#[allow(dead_code)]
pub enum AccountIdx {
    NftOwner = 0,
    NftAccount = 1,
    PersonalPosition = 2,
    PoolState = 3,
    ProtocolPosition = 4,
    TokenAccount0 = 5,
    TokenAccount1 = 6,
    TokenVault0 = 7,
    TokenVault1 = 8,
    TickArrayLower = 9,
    TickArrayUpper = 10,
    RecipientToken0 = 11,
    RecipientToken1 = 12,
    TokenProgram = 13,
    TokenProgram2022 = 14,
    MemoProgram = 15,
    VaultMint0 = 16,
    VaultMint1 = 17,
}

const MIN_ACCOUNTS: usize = AccountIdx::VaultMint1 as usize + 1;

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < MIN_ACCOUNTS {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let _args = DecreaseLiquidityArgs::try_from_bytes(data)?;
    let _accounts = accounts;
    Err(ProgramError::InvalidInstructionData)
}
