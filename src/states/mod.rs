
pub mod config;
pub mod oracle;
pub mod personal_position;
pub mod pool;
pub mod pool_fee;
pub mod protocol_position;
pub mod tick_array;
pub mod tickarray_bitmap_extension;

pub use config::*;
pub use oracle::*;
pub use personal_position::*;
pub use pool::*;
pub use pool_fee::*;
pub use protocol_position::*;
pub use tick_array::*;
pub use tickarray_bitmap_extension::*;

#[cfg(test)]
mod layout_tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn struct_sizes_match_anchor_lens() {
        assert_eq!(size_of::<AmmConfig>() + 8, AmmConfig::LEN);
        assert_eq!(size_of::<PoolState>() + 8, PoolState::LEN);
        assert_eq!(size_of::<TickArrayState>() + 8, TickArrayState::LEN);
        assert_eq!(size_of::<ObservationState>() + 8, ObservationState::LEN);
        assert_eq!(
            size_of::<TickArrayBitmapExtension>() + 8,
            TickArrayBitmapExtension::LEN
        );
    }

    #[test]
    fn embedded_struct_sizes() {
        assert_eq!(size_of::<DynamicFeeInfo>(), DynamicFeeInfo::LEN);
        assert_eq!(size_of::<RewardInfo>(), RewardInfo::LEN);
        assert_eq!(size_of::<TickState>(), TickState::LEN);
        assert_eq!(size_of::<Observation>(), Observation::LEN);
    }
}
