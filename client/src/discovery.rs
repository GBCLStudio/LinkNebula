use common::hal::Hardware;
use common::protocol::{Beacon, NodeId, PacketType};
use core::time::Duration;

/// 尝试发现网络中的服务器节点
pub fn find_server<H: Hardware>(hardware: &mut H) -> Option<NodeId> {
    println!("开始寻找服务器节点...");
    
    // 最多尝试30秒
    let max_attempts = 30;
    let mut attempt = 0;
    
    while attempt < max_attempts {
        // 发送广播信标
        send_discovery_beacon(hardware);
        
        // 尝试接收服务器响应
        if let Some(server_id) = receive_server_response(hardware) {
            return Some(server_id);
        }
        
        // 等待1秒再尝试
        let _ = hardware.delay_ms(1000);
        attempt += 1;
        println!("搜索服务器中... {}/{}s", attempt, max_attempts);
    }
    
    println!("未找到服务器节点");
    None
}

/// 发送发现信标
fn send_discovery_beacon<H: Hardware>(hardware: &mut H) {
    let node_id = hardware.get_node_id();
    let battery_level = hardware.get_battery_level().unwrap_or(100);
    let rssi = hardware.get_radio().get_rssi().unwrap_or(-80);
    
    // 创建信标
    let beacon = Beacon::new(node_id, battery_level, rssi);
    
    // 发送信标
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_beacon(&beacon) {
        println!("发送发现信标失败: {:?}", e);
    }
}

/// 接收服务器响应
fn receive_server_response<H: Hardware>(hardware: &mut H) -> Option<NodeId> {
    // 尝试接收信标
    let radio = hardware.get_radio();
    if let Ok(Some(beacon)) = radio.receive_beacon() {
        // 验证是否是服务器节点
        if beacon.is_valid() && beacon.packet_type == PacketType::Beacon as u8 {
            // 实际项目中可能需要更复杂的验证逻辑
            println!("发现潜在服务器节点，RSSI: {}", beacon.rssi);
            return Some(NodeId(beacon.source));
        }
    }
    
    None
} 