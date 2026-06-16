
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub struct CreatePoolArgs {
    pub sqrt_price_x64: u128,
    pub open_time: u64,
}

impl CreatePoolArgs {
    pub const SIZE: usize = 16 + 8;

    #[inline(always)]
    pub fn try_from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::SIZE {
            return Err(ProgramError::InvalidInstructionData);
        }
        let p = data.as_ptr();
        let sqrt_price_x64 =
            u128::from_le_bytes(unsafe { *(p as *const [u8; 16]) });
        let open_time =
            u64::from_le_bytes(unsafe { *(p.add(16) as *const [u8; 8]) });
        Ok(Self {
            sqrt_price_x64,
            open_time,
        })
    }
}

#[repr(usize)]
#[allow(dead_code)]
pub enum AccountIdx {
    PoolCreator = 0,
    AmmConfig = 1,
    PoolState = 2,
    TokenMint0 = 3,
    TokenMint1 = 4,
    TokenVault0 = 5,
    TokenVault1 = 6,
    ObservationState = 7,
    TickArrayBitmapExtension = 8,
    TokenProgram0 = 9,
    TokenProgram1 = 10,
    SystemProgram = 11,
    Rent = 12,
}

const MIN_ACCOUNTS: usize = AccountIdx::Rent as usize + 1;

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < MIN_ACCOUNTS {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let _args = CreatePoolArgs::try_from_bytes(data)?;
    let _accounts = accounts;
    Err(ProgramError::InvalidInstructionData)
}
