#![no_std]
use zerocopy::{AsBytes, FromBytes};

/// 网络层统一封包格式
#[repr(C, packed)]
#[derive(AsBytes, FromBytes)]
pub struct NetworkPacket {
    pub header: PacketHeader,
    pub payload: [u8; 252], // 总长度256字节
}

/// 协议头部定义
#[repr(C, packed)]
#[derive(AsBytes, FromBytes)]
pub struct PacketHeader {
    pub magic: u16,        // 0xAA55
    pub version: u8,       // 0x01
    pub packet_type: PacketType,
    pub ttl: u8,
    pub src_mac: [u8; 6],
    pub dest_mac: [u8; 6],
    pub checksum: u32,
}

/// 信标负载结构，用于零拷贝从NetworkPacket中提取
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BeaconPayload {
    /// 协议版本
    pub version: u8,
    /// 包类型
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

pub mod beacon;
pub mod data;

pub use beacon::Beacon;
pub use data::DataPacket;

// 协议常量和公共类型定义
pub const MAX_PACKET_SIZE: usize = 256;
pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Beacon = 0x01,
    Data = 0x02,
    Ack = 0x03,
    Control = 0x04,
    ServiceRequest = 0x05, // 服务请求
    ServiceResponse = 0x06, // 服务响应
    PathEstablish = 0x07,  // 路径建立
    PathConfirm = 0x08,    // 路径确认
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(pub [u8; 6]);

impl NodeId {
    pub const BROADCAST: Self = Self([0xFF; 6]);
    
    pub fn new(id: [u8; 6]) -> Self {
        Self(id)
    }
    
    pub fn is_broadcast(&self) -> bool {
        self.0 == Self::BROADCAST.0
    }
}

// 服务类型定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ServiceType {
    Storage = 0x01,       // 存储服务
    Processing = 0x02,    // 处理服务
    Gateway = 0x03,       // 网关服务
    VideoRelay = 0x04,    // 视频中继服务
    AudioRelay = 0x05,    // 音频中继服务
    DataRelay = 0x06,     // 数据中继服务
    SensorCollection = 0x07, // 传感器数据收集
}

// 服务质量要求
#[derive(Debug, Clone, Copy)]
pub struct QosRequirements {
    pub min_bandwidth: u16,  // 最小带宽要求 (kbps)
    pub max_latency: u16,    // 最大延迟 (ms)
    pub reliability: u8,     // 可靠性要求 (0-100)
}

// 服务请求包
#[derive(Debug)]
pub struct ServiceRequest {
    pub service_type: ServiceType,      // 请求的服务类型
    pub qos: QosRequirements,           // 服务质量要求
    pub expiry_time: u32,               // 服务过期时间 (秒)
}

// 服务响应包
#[derive(Debug)]
pub struct ServiceResponse {
    pub service_id: u32,                // 服务ID
    pub server_node_id: NodeId,         // 服务器节点ID
    pub status: u8,                     // 状态(0=成功, 1=失败, 2=部分满足)
}

// 路径建立状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PathStatus {
    Success = 0x00,        // 成功建立
    NoResource = 0x01,     // 资源不足
    QosNotMet = 0x02,      // 无法满足QoS要求
    Timeout = 0x03,        // 超时
    ServerBusy = 0x04,     // 服务器忙
}

impl NetworkPacket {
    /// 零拷贝转换信标包
    pub fn as_beacon(&self) -> Option<&BeaconPayload> {
        if self.header.packet_type == PacketType::Beacon {
            Some(unsafe { &*(&self.payload as *const _ as *const BeaconPayload) })
        } else {
            None
        }
    }
}

// 序列化/反序列化工具函数
pub fn serialize_service_request(request: &ServiceRequest, buffer: &mut [u8]) -> usize {
    if buffer.len() < 8 {
        return 0;
    }
    
    buffer[0] = request.service_type as u8;
    
    // 序列化QoS需求
    let bandwidth_bytes = request.qos.min_bandwidth.to_be_bytes();
    buffer[1] = bandwidth_bytes[0];
    buffer[2] = bandwidth_bytes[1];
    
    let latency_bytes = request.qos.max_latency.to_be_bytes();
    buffer[3] = latency_bytes[0];
    buffer[4] = latency_bytes[1];
    
    buffer[5] = request.qos.reliability;
    
    // 序列化过期时间
    let expiry_bytes = request.expiry_time.to_be_bytes();
    buffer[6] = expiry_bytes[0];
    buffer[7] = expiry_bytes[1];
    
    8
}

pub fn deserialize_service_request(buffer: &[u8]) -> Option<ServiceRequest> {
    if buffer.len() < 8 {
        return None;
    }
    
    let service_type = match buffer[0] {
        0x01 => ServiceType::Storage,
        0x02 => ServiceType::Processing,
        0x03 => ServiceType::Gateway,
        0x04 => ServiceType::VideoRelay,
        0x05 => ServiceType::AudioRelay,
        0x06 => ServiceType::DataRelay,
        0x07 => ServiceType::SensorCollection,
        _ => return None,
    };
    
    // 反序列化QoS需求
    let min_bandwidth = u16::from_be_bytes([buffer[1], buffer[2]]);
    let max_latency = u16::from_be_bytes([buffer[3], buffer[4]]);
    let reliability = buffer[5];
    
    // 反序列化过期时间
    let expiry_time = u32::from_be_bytes([buffer[6], buffer[7], 0, 0]);
    
    Some(ServiceRequest {
        service_type,
        qos: QosRequirements {
            min_bandwidth,
            max_latency,
            reliability,
        },
        expiry_time,
    })
}

pub fn serialize_service_response(response: &ServiceResponse, buffer: &mut [u8]) -> usize {
    if buffer.len() < 11 {
        return 0;
    }
    
    // 序列化服务ID
    let service_id_bytes = response.service_id.to_be_bytes();
    buffer[0] = service_id_bytes[0];
    buffer[1] = service_id_bytes[1];
    buffer[2] = service_id_bytes[2];
    buffer[3] = service_id_bytes[3];
    
    // 序列化服务器节点ID
    buffer[4..10].copy_from_slice(&response.server_node_id.0);
    
    // 序列化状态
    buffer[10] = response.status;
    
    11
}

pub fn deserialize_service_response(buffer: &[u8]) -> Option<ServiceResponse> {
    if buffer.len() < 11 {
        return None;
    }
    
    // 反序列化服务ID
    let service_id = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
    
    // 反序列化服务器节点ID
    let mut server_node_id = [0u8; 6];
    server_node_id.copy_from_slice(&buffer[4..10]);
    
    // 反序列化状态
    let status = buffer[10];
    
    Some(ServiceResponse {
        service_id,
        server_node_id: NodeId(server_node_id),
        status,
    })
}