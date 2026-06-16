use bytemuck::{Pod, Zeroable};

pub const MAX_FEE_RATE_NUMERATOR: u32 = 100_000;

pub const VOLATILITY_ACCUMULATOR_SCALE: u16 = 10_000;
pub const REDUCTION_FACTOR_DENOMINATOR: u16 = 10_000;
pub const DYNAMIC_FEE_CONTROL_DENOMINATOR: u32 = 100_000;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DynamicFeeInfo {
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub dynamic_fee_control: u32,
    pub max_volatility_accumulator: u32,

    pub tick_spacing_index_reference: i32,
    pub volatility_reference: u32,
    pub volatility_accumulator: u32,
    pub last_update_timestamp: u64,
    pub padding: [u8; 46],
}

impl DynamicFeeInfo {
    pub const LEN: usize = 2 + 2 + 2 + 4 + 4 + 4 + 4 + 4 + 8 + 46;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CollectFeeOn {
    FromInput = 0,
    Token0Only = 1,
    Token1Only = 2,
}

impl CollectFeeOn {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Token0Only,
            2 => Self::Token1Only,
            _ => Self::FromInput,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
