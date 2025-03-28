#![no_std]
#![cfg_attr(feature = "bearpi", no_main)]

pub mod protocol;
pub mod hal;
pub mod utils;

// 重新导出核心模块
pub use protocol::{Beacon, DataPacket};
pub use hal::{Hardware, RadioInterface};
pub use utils::{AlignedBuffer, calculate_checksum}; 