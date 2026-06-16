
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub struct IncreaseLiquidityArgs {
    pub liquidity: u128,
    pub amount_0_max: u64,
    pub amount_1_max: u64,
    pub base_flag: Option<bool>,
}

impl IncreaseLiquidityArgs {
    pub const MIN_SIZE: usize = 16 + 8 + 8 + 1;

    #[inline(always)]
    pub fn try_from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::MIN_SIZE {
            return Err(ProgramError::InvalidInstructionData);
        }
        let p = data.as_ptr();
        let liquidity = u128::from_le_bytes(unsafe { *(p as *const [u8; 16]) });
        let amount_0_max = u64::from_le_bytes(unsafe { *(p.add(16) as *const [u8; 8]) });
        let amount_1_max = u64::from_le_bytes(unsafe { *(p.add(24) as *const [u8; 8]) });
        let base_flag = match data[32] {
            0 => None,
            1 => {
                if data.len() < 34 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Some(data[33] != 0)
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        };
        Ok(Self {
            liquidity,
            amount_0_max,
            amount_1_max,
            base_flag,
        })
    }
}

#[repr(usize)]
#[allow(dead_code)]
pub enum AccountIdx {
    NftOwner = 0,
    NftAccount = 1,
    PoolState = 2,
    ProtocolPosition = 3,
    PersonalPosition = 4,
    TickArrayLower = 5,
    TickArrayUpper = 6,
    TokenAccount0 = 7,
    TokenAccount1 = 8,
    TokenVault0 = 9,
    TokenVault1 = 10,
    TokenProgram = 11,
    TokenProgram2022 = 12,
    VaultMint0 = 13,
    VaultMint1 = 14,
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
    let _args = IncreaseLiquidityArgs::try_from_bytes(data)?;
    let _accounts = accounts;
    Err(ProgramError::InvalidInstructionData)
}
