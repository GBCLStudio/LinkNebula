use crate::protocol::{NodeId, PacketType, PROTOCOL_VERSION};
use crate::utils::calculate_checksum;

/// 网络信标包，用于发现和维护网络拓扑
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Beacon {
    /// 协议版本
    pub version: u8,
    /// 数据包类型（固定为Beacon）
    pub packet_type: u8,
    /// 源节点ID
    pub source: [u8; 6],
    /// 电池电量（百分比）
    pub battery_level: u8,
    /// 信号强度指示
    pub rssi: i8,
    /// 路由跳数
    pub hop_count: u8,
    /// 预留字段
    pub reserved: [u8; 3],
    /// 校验和
    pub checksum: u16,
}

impl Beacon {
    pub fn new(source: NodeId, battery_level: u8, rssi: i8) -> Self {
        let mut beacon = Self {
            version: PROTOCOL_VERSION,
            packet_type: PacketType::Beacon as u8,
            source: source.0,
            battery_level,
            rssi,
            hop_count: 0,
            reserved: [0; 3],
            checksum: 0, // 临时值
        };
        
        // 计算校验和
        beacon.update_checksum();
        beacon
    }
    
    pub fn update_checksum(&mut self) {
        // 设置校验和为0进行计算
        self.checksum = 0;
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        };
        self.checksum = calculate_checksum(data);
    }
    
    pub fn is_valid(&self) -> bool {
        let mut copy = *self;
        copy.checksum = 0;
        let data = unsafe {
            core::slice::from_raw_parts(
                &copy as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        };
        calculate_checksum(data) == self.checksum
    }
} 