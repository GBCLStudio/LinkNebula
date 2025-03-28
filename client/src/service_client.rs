use common::protocol::{NodeId, DataPacket, ServiceType, QosRequirements, PacketType};
use common::protocol::{ServiceRequest, ServiceResponse, serialize_service_request, deserialize_service_response};
use common::hal::Hardware;
use common::utils::AlignedBuffer;

/// 服务端点，表示可以连接的远程服务
#[derive(Debug, Clone, Copy)]
pub struct ServiceEndpoint {
    /// 服务ID
    pub service_id: u32,
    /// 服务器节点ID
    pub server_id: NodeId,
    /// 中继节点ID（转发节点）
    pub relay_id: NodeId,
    /// 服务类型
    pub service_type: ServiceType,
    /// 跳数
    pub hops: u8,
}

/// 请求服务，与转发节点通信，获取合适的服务端点
pub fn request_service<H: Hardware>(
    hardware: &mut H,
    forward_id: NodeId,
    service_type: ServiceType,
    qos: &QosRequirements,
    expiry_time: u32,
    tx_buffer: &mut AlignedBuffer<256>,
    rx_buffer: &mut AlignedBuffer<1024>
) -> Option<ServiceEndpoint> {
    println!("请求服务：类型={:?}, 转发节点={:?}", service_type, forward_id);
    
    // 创建服务请求
    let service_request = ServiceRequest {
        service_type,
        qos: *qos,
        expiry_time,
    };
    
    // 序列化请求
    let tx_data = tx_buffer.as_mut_slice();
    let request_len = serialize_service_request(&service_request, tx_data);
    
    if request_len == 0 {
        println!("序列化服务请求失败");
        return None;
    }
    
    // 创建请求数据包
    let node_id = hardware.get_node_id();
    let request_packet = DataPacket::new(
        node_id,
        forward_id,
        0, // 包ID
        &tx_data[..request_len]
    );
    
    // 发送请求
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_data(&request_packet) {
        println!("发送服务请求失败: {:?}", e);
        return None;
    }
    
    println!("已发送服务请求，等待响应...");
    
    // 等待响应（最多等待10秒）
    let mut retry_count = 0;
    const MAX_RETRIES: u8 = 10;
    
    while retry_count < MAX_RETRIES {
        // 尝试接收数据
        let buffer = rx_buffer.as_mut_slice();
        if let Ok(Some(packet)) = radio.receive_data(buffer) {
            let source = NodeId(packet.header.source);
            
            // 检查是否是来自转发节点的响应
            if source == forward_id && packet.header.packet_type == PacketType::ServiceResponse {
                // 尝试解析服务响应
                if let Some(response) = deserialize_service_response(packet.data) {
                    if response.status == 0 { // 成功
                        println!("收到成功的服务响应: 服务器={:?}, 服务ID={}", 
                                 response.server_node_id, response.service_id);
                        
                        // 创建服务端点
                        return Some(ServiceEndpoint {
                            service_id: response.service_id,
                            server_id: response.server_node_id,
                            relay_id: forward_id,
                            service_type,
                            hops: 0, // 初始值，将在路径确认中更新
                        });
                    } else {
                        println!("服务响应表示失败，状态: {}", response.status);
                        return None;
                    }
                }
            }
        }
        
        // 等待1秒后重试
        let _ = hardware.delay_ms(1000);
        retry_count += 1;
    }
    
    println!("等待服务响应超时");
    None
}

/// 更新服务端点（例如更新跳数信息）
pub fn update_service_endpoint(endpoint: &mut ServiceEndpoint, hops: u8) {
    endpoint.hops = hops;
}

/// 关闭服务连接
pub fn close_service<H: Hardware>(
    hardware: &mut H,
    endpoint: &ServiceEndpoint,
    tx_buffer: &mut AlignedBuffer<256>
) -> bool {
    println!("关闭服务连接: 服务ID={}, 服务器={:?}", 
             endpoint.service_id, endpoint.server_id);
    
    // 创建关闭服务请求
    let mut close_data = [0u8; 6];
    
    // 0-3: 服务ID
    let service_id_bytes = endpoint.service_id.to_be_bytes();
    close_data[0..4].copy_from_slice(&service_id_bytes);
    
    // 4: 关闭原因（0=正常关闭）
    close_data[4] = 0;
    
    // 5: 预留
    close_data[5] = 0;
    
    // 创建关闭请求数据包
    let node_id = hardware.get_node_id();
    let close_packet = DataPacket::new(
        node_id,
        endpoint.relay_id, // 发送给中继节点
        0, // 包ID
        &close_data
    );
    
    // 发送关闭请求
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_data(&close_packet) {
        println!("发送服务关闭请求失败: {:?}", e);
        return false;
    }
    
    true
} 