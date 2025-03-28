use crate::protocol::{NodeId, PacketType, PROTOCOL_VERSION, MAX_PACKET_SIZE};
use crate::utils::calculate_checksum;

/// 数据包头部
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DataHeader {
    /// 协议版本
    pub version: u8,
    /// 数据包类型（固定为Data）
    pub packet_type: u8,
    /// 源节点ID
    pub source: [u8; 6],
    /// 目标节点ID
    pub destination: [u8; 6],
    /// 数据包ID
    pub packet_id: u16,
    /// 总分片数
    pub total_fragments: u8,
    /// 当前分片索引
    pub fragment_index: u8,
    /// 数据长度
    pub data_length: u16,
    /// 校验和
    pub checksum: u16,
}

/// 数据包，采用零拷贝设计
#[derive(Debug)]
pub struct DataPacket<'a> {
    pub header: DataHeader,
    pub data: &'a [u8],
}

impl<'a> DataPacket<'a> {
    pub fn new(source: NodeId, destination: NodeId, packet_id: u16, data: &'a [u8]) -> Self {
        assert!(data.len() <= MAX_PACKET_SIZE - core::mem::size_of::<DataHeader>());
        
        let mut header = DataHeader {
            version: PROTOCOL_VERSION,
            packet_type: PacketType::Data as u8,
            source: source.0,
            destination: destination.0,
            packet_id,
            total_fragments: 1,
            fragment_index: 0,
            data_length: data.len() as u16,
            checksum: 0, // 临时值
        };
        
        let mut packet = Self { header, data };
        packet.update_checksum();
        packet
    }
    
    pub fn update_checksum(&mut self) {
        // 设置校验和为0进行计算
        self.header.checksum = 0;
        
        // 首先计算头部的校验和
        let header_data = unsafe {
            core::slice::from_raw_parts(
                &self.header as *const DataHeader as *const u8,
                core::mem::size_of::<DataHeader>(),
            )
        };
        
        // 然后包含数据部分
        let mut checksum = calculate_checksum(header_data);
        let data_checksum = calculate_checksum(self.data);
        
        // 合并校验和
        self.header.checksum = checksum ^ data_checksum;
    }
    
    pub fn is_valid(&self) -> bool {
        let mut header_copy = self.header;
        header_copy.checksum = 0;
        
        let header_data = unsafe {
            core::slice::from_raw_parts(
                &header_copy as *const DataHeader as *const u8,
                core::mem::size_of::<DataHeader>(),
            )
        };
        
        let header_checksum = calculate_checksum(header_data);
        let data_checksum = calculate_checksum(self.data);
        
        (header_checksum ^ data_checksum) == self.header.checksum
    }
} 