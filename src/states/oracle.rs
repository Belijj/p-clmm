use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

pub const OBSERVATION_SEED: &[u8] = b"observation";
pub const OBSERVATION_NUM: usize = 100;
pub const OBSERVATION_UPDATE_DURATION_DEFAULT: u32 = 15;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct Observation {
    pub block_timestamp: u32,
    pub tick_cumulative: i64,
    pub padding: [u64; 4],
}

impl Observation {
    pub const LEN: usize = 4 + 8 + 8 * 4;
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ObservationState {
    pub initialized: u8,
    pub recent_epoch: u64,
    pub observation_index: u16,
    pub pool_id: Pubkey,
    pub observations: [Observation; OBSERVATION_NUM],
    pub padding: [u64; 4],
}

impl crate::util::AccountSchema for ObservationState {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::OBSERVATION_STATE;
}

impl ObservationState {
    pub const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::OBSERVATION_STATE;

    pub const LEN: usize = 8 + 1 + 8 + 2 + 32 + (Observation::LEN * OBSERVATION_NUM) + 8 * 4;
}
