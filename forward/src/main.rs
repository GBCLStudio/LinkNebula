#![cfg_attr(not(feature = "simulator"), no_std)]
#![cfg_attr(not(feature = "simulator"), no_main)]

mod routing;
mod directory;

use common::protocol::{Beacon, DataPacket, NodeId, ServiceType, ServiceRequest, ServiceResponse, QosRequirements, PathStatus};
use common::protocol::{PacketType, deserialize_service_request, serialize_service_response};
use common::hal::Hardware;
use common::utils::AlignedBuffer;
use routing::dynamic_forwarding::ForwardingEngine;
use directory::election::ElectionProtocol;
use directory::service_directory::{NetworkServiceDirectory, Capabilities, ServiceMetrics};

#[cfg(feature = "simulator")]
fn main() {
    // 模拟器入口
    use common::hal::simulator::{SimChannel, SimHardware};
    use std::thread;
    use std::time::Duration;
    
    println!("启动AetherLink转发节点（模拟器模式）");
    
    let channel = SimChannel::new();
    let node_id = NodeId::new([0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6]);
    let mut hardware = SimHardware::new(node_id, channel);
    
    forward_main(&mut hardware);
}

#[cfg(feature = "bearpi")]
#[cortex_m_rt::entry]
fn main() -> ! {
    // BearPi硬件入口
    use common::hal::bearpi_hi2821::BearPiHardware;
    
    // 初始化BearPi硬件
    let node_id = NodeId::new([0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6]);
    let mut hardware = BearPiHardware::new(node_id);
    
    forward_main(&mut hardware);
    
    // 嵌入式设备不应该退出主循环
    loop {
        // 无限循环避免退出
    }
}

fn forward_main<H: Hardware>(hardware: &mut H) {
    // 配置无线电
    let radio = hardware.get_radio();
    let _ = radio.configure(15, 20); // 使用15号信道，20dBm发射功率
    
    // 初始化转发引擎
    let mut forwarding_engine = ForwardingEngine::new(hardware.get_node_id());
    
    // 初始化选举协议
    let mut election = ElectionProtocol::new(hardware.get_node_id());
    
    // 初始化服务目录
    let mut service_directory = NetworkServiceDirectory::new();
    
    // 创建缓冲区
    let mut rx_buffer = AlignedBuffer::<1024>::new();
    let mut tx_buffer = AlignedBuffer::<256>::new();
    let mut beacon_timer: u64 = 0;
    let mut election_timer: u64 = 0;
    let mut directory_cleanup_timer: u64 = 0;
    
    println!("转发节点启动完成，开始执行主循环");
    
    // 主循环
    loop {
        // 获取当前时间
        let now = hardware.get_timestamp_ms().unwrap_or(0);
        
        // 每60秒广播一次信标
        if now - beacon_timer > 60000 {
            send_beacon(hardware);
            beacon_timer = now;
        }
        
        // 每5分钟执行一次主服务器选举
        if now - election_timer > 300000 {
            election.initiate_election(hardware);
            election_timer = now;
        }
        
        // 清理过期的服务条目
        if now - directory_cleanup_timer > 30000 {
            service_directory.cleanup(now);
            directory_cleanup_timer = now;
        }
        
        // 接收数据包
        let radio = hardware.get_radio();
        let buffer = rx_buffer.as_mut_slice();
        
        if let Ok(Some(packet)) = radio.receive_data(buffer) {
            // 处理各种数据包
            match packet.header.packet_type {
                PacketType::Data => {
                    handle_data_packet(hardware, &mut forwarding_engine, &packet);
                },
                PacketType::ServiceRequest => {
                    handle_service_request(hardware, &mut service_directory, &mut forwarding_engine, 
                                          &packet, &mut tx_buffer, now);
                },
                PacketType::PathEstablish => {
                    handle_path_establish(hardware, &mut forwarding_engine, &packet, &mut tx_buffer);
                },
                PacketType::PathConfirm => {
                    handle_path_confirm(hardware, &mut forwarding_engine, &packet, &mut tx_buffer);
                },
                _ => {
                    // 处理其他类型的数据包
                    handle_other_packet(hardware, &mut forwarding_engine, &packet);
                }
            }
        }
        
        // 接收信标
        if let Ok(Some(beacon)) = radio.receive_beacon() {
            handle_beacon(hardware, &mut forwarding_engine, &mut service_directory, &beacon, now);
        }
        
        // 处理选举消息
        election.process_messages(hardware);
        
        // 每1秒钟做一次延迟，可以根据实际硬件调整
        let _ = hardware.delay_ms(1000);
    }
}

/// 发送本节点信标
fn send_beacon<H: Hardware>(hardware: &mut H) {
    let node_id = hardware.get_node_id();
    let battery_level = hardware.get_battery_level().unwrap_or(100);
    let rssi = hardware.get_radio().get_rssi().unwrap_or(-80);
    
    // 创建信标
    let beacon = Beacon::new(node_id, battery_level, rssi);
    
    // 发送信标
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_beacon(&beacon) {
        println!("发送信标失败: {:?}", e);
    } else {
        println!("发送转发节点信标，电池电量: {}%", battery_level);
    }
}

/// 处理接收到的信标
fn handle_beacon<H: Hardware>(
    hardware: &mut H,
    forwarding_engine: &mut ForwardingEngine,
    service_directory: &mut NetworkServiceDirectory,
    beacon: &Beacon,
    current_time: u64
) {
    if beacon.is_valid() {
        let source = NodeId(beacon.source);
        
        // 更新路由表
        forwarding_engine.update_route(source, beacon.rssi);
        
        println!("接收到来自 {:?} 的信标，信号强度: {}, 电池电量: {}%",
            source, beacon.rssi, beacon.battery_level);
            
        // 如果是服务器节点信标，更新服务目录
        // 这里简单地假设所有信标都可能是来自服务器的
        // 实际中应该有更多的判断逻辑
        let capabilities = Capabilities {
            max_bandwidth: 1000, // 默认1 Mbps
            min_latency: 100,    // 默认100ms
            reliability: 90,     // 默认90%
            battery_level: beacon.battery_level,
        };
        
        let metrics = ServiceMetrics {
            success_rate: 100,     // 默认100%
            avg_response_time: 50, // 默认50ms
            signal_strength: beacon.rssi,
        };
        
        // 更新所有可能的服务类型（简化处理，实际中应该根据信标内容确定支持的服务）
        service_directory.update_service(
            source,
            ServiceType::VideoRelay,
            0, // 假设负载为0
            capabilities,
            metrics,
            current_time
        );
    }
}

/// 处理接收到的数据包
fn handle_data_packet<H: Hardware>(
    hardware: &mut H,
    forwarding_engine: &mut ForwardingEngine,
    packet: &DataPacket
) {
    let source = NodeId(packet.header.source);
    let destination = NodeId(packet.header.destination);
    
    println!("接收到来自 {:?} 发往 {:?} 的数据包，大小: {} 字节",
        source, destination, packet.data.len());
    
    // 转发数据包
    if !destination.is_broadcast() && destination != hardware.get_node_id() {
        if let Some(next_hop) = forwarding_engine.get_next_hop(destination) {
            println!("转发数据包到下一跳: {:?}", next_hop);
            
            // 创建新的数据包进行转发
            let node_id = hardware.get_node_id();
            let forward_packet = DataPacket::new(
                node_id,
                next_hop,
                packet.header.packet_id,
                packet.data
            );
            
            // 发送转发的数据包
            let radio = hardware.get_radio();
            if let Err(e) = radio.send_data(&forward_packet) {
                println!("转发数据包失败: {:?}", e);
            }
        } else {
            println!("未找到到达 {:?} 的路由，丢弃数据包", destination);
        }
    }
}

/// 处理服务请求数据包
fn handle_service_request<H: Hardware>(
    hardware: &mut H,
    service_directory: &mut NetworkServiceDirectory,
    forwarding_engine: &mut ForwardingEngine,
    packet: &DataPacket,
    tx_buffer: &mut AlignedBuffer<256>,
    current_time: u64
) {
    let source = NodeId(packet.header.source);
    
    println!("接收到来自 {:?} 的服务请求", source);
    
    // 反序列化服务请求
    if let Some(service_request) = deserialize_service_request(packet.data) {
        println!("请求的服务类型: {:?}", service_request.service_type);
        
        // 查询服务目录，寻找最佳服务提供者
        if let Some(best_service) = service_directory.find_best_service(
            service_request.service_type, 
            &service_request.qos
        ) {
            println!("找到最佳服务提供者: {:?}", best_service.node_id);
            
            // 创建服务响应
            let service_response = ServiceResponse {
                service_id: current_time as u32, // 使用时间戳作为服务ID
                server_node_id: best_service.node_id,
                status: 0, // 成功
            };
            
            // 序列化响应
            let tx_data = tx_buffer.as_mut_slice();
            let response_len = serialize_service_response(&service_response, tx_data);
            
            if response_len > 0 {
                // 创建响应数据包
                let node_id = hardware.get_node_id();
                let response_packet = DataPacket::new(
                    node_id,
                    source,
                    packet.header.packet_id,
                    &tx_data[..response_len]
                );
                
                // 发送响应
                let radio = hardware.get_radio();
                if let Err(e) = radio.send_data(&response_packet) {
                    println!("发送服务响应失败: {:?}", e);
                } else {
                    println!("已发送服务响应给 {:?}", source);
                }
                
                // 向最佳服务器发送路径建立请求
                establish_path(hardware, source, best_service.node_id, 
                              service_request.service_type, &service_request.qos,
                              tx_buffer);
            }
        } else {
            println!("未找到匹配的服务提供者");
            
            // 创建失败响应
            let service_response = ServiceResponse {
                service_id: 0,
                server_node_id: NodeId::BROADCAST, // 使用广播地址表示未找到
                status: 1, // 失败
            };
            
            // 序列化响应
            let tx_data = tx_buffer.as_mut_slice();
            let response_len = serialize_service_response(&service_response, tx_data);
            
            if response_len > 0 {
                // 创建响应数据包
                let node_id = hardware.get_node_id();
                let response_packet = DataPacket::new(
                    node_id,
                    source,
                    packet.header.packet_id,
                    &tx_data[..response_len]
                );
                
                // 发送响应
                let radio = hardware.get_radio();
                if let Err(e) = radio.send_data(&response_packet) {
                    println!("发送服务失败响应失败: {:?}", e);
                }
            }
        }
    } else {
        println!("无法解析服务请求数据");
    }
}

/// 建立中继路径
fn establish_path<H: Hardware>(
    hardware: &mut H,
    client: NodeId,
    server: NodeId,
    service_type: ServiceType,
    qos: &QosRequirements,
    tx_buffer: &mut AlignedBuffer<256>
) {
    println!("建立从 {:?} 到 {:?} 的中继路径", client, server);
    
    // 创建路径建立请求数据
    let mut path_data = [0u8; 20];
    
    // 填充路径建立请求
    // 0-5: 客户端节点ID
    path_data[0..6].copy_from_slice(&client.0);
    
    // 6: 服务类型
    path_data[6] = service_type as u8;
    
    // 7-8: 最小带宽
    let bandwidth_bytes = qos.min_bandwidth.to_be_bytes();
    path_data[7] = bandwidth_bytes[0];
    path_data[8] = bandwidth_bytes[1];
    
    // 9-10: 最大延迟
    let latency_bytes = qos.max_latency.to_be_bytes();
    path_data[9] = latency_bytes[0];
    path_data[10] = latency_bytes[1];
    
    // 11: 可靠性
    path_data[11] = qos.reliability;
    
    // 创建发往服务器的路径建立数据包
    let node_id = hardware.get_node_id();
    let path_packet = DataPacket::new(
        node_id,
        server,
        0, // 新包ID
        &path_data
    );
    
    // 发送路径建立请求
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_data(&path_packet) {
        println!("发送路径建立请求失败: {:?}", e);
    } else {
        println!("已发送路径建立请求给服务器 {:?}", server);
    }
}

/// 处理路径建立数据包
fn handle_path_establish<H: Hardware>(
    hardware: &mut H,
    forwarding_engine: &mut ForwardingEngine,
    packet: &DataPacket,
    tx_buffer: &mut AlignedBuffer<256>
) {
    let source = NodeId(packet.header.source);
    let destination = NodeId(packet.header.destination);
    
    println!("接收到来自 {:?} 的路径建立请求", source);
    
    if destination != hardware.get_node_id() {
        // 如果不是发给本节点的，转发
        if let Some(next_hop) = forwarding_engine.get_next_hop(destination) {
            // 创建新的数据包进行转发
            let node_id = hardware.get_node_id();
            let forward_packet = DataPacket::new(
                node_id,
                next_hop,
                packet.header.packet_id,
                packet.data
            );
            
            // 发送转发的数据包
            let radio = hardware.get_radio();
            if let Err(e) = radio.send_data(&forward_packet) {
                println!("转发路径建立请求失败: {:?}", e);
            } else {
                println!("已转发路径建立请求到 {:?}", next_hop);
            }
        }
    } else {
        // 本节点是服务器，处理路径建立请求
        if packet.data.len() >= 12 {
            // 提取客户端ID
            let mut client_id = [0u8; 6];
            client_id.copy_from_slice(&packet.data[0..6]);
            let client = NodeId(client_id);
            
            // 生成路径确认响应
            let mut confirm_data = [0u8; 8];
            
            // 0-5: 客户端节点ID
            confirm_data[0..6].copy_from_slice(&client.0);
            
            // 6: 路径状态
            confirm_data[6] = PathStatus::Success as u8;
            
            // 7: 跳数
            confirm_data[7] = 1; // 假设只有一跳
            
            // 创建确认数据包
            let node_id = hardware.get_node_id();
            let confirm_packet = DataPacket::new(
                node_id,
                source, // 发送给转发节点
                packet.header.packet_id,
                &confirm_data
            );
            
            // 发送确认
            let radio = hardware.get_radio();
            if let Err(e) = radio.send_data(&confirm_packet) {
                println!("发送路径确认失败: {:?}", e);
            } else {
                println!("已发送路径确认给转发节点 {:?}", source);
            }
        }
    }
}

/// 处理路径确认数据包
fn handle_path_confirm<H: Hardware>(
    hardware: &mut H,
    forwarding_engine: &mut ForwardingEngine,
    packet: &DataPacket,
    tx_buffer: &mut AlignedBuffer<256>
) {
    let source = NodeId(packet.header.source);
    
    println!("接收到来自 {:?} 的路径确认", source);
    
    if packet.data.len() >= 8 {
        // 提取客户端ID
        let mut client_id = [0u8; 6];
        client_id.copy_from_slice(&packet.data[0..6]);
        let client = NodeId(client_id);
        
        // 提取路径状态
        let status = packet.data[6];
        
        // 提取跳数
        let hops = packet.data[7];
        
        println!("路径确认：客户端={:?}, 状态={}, 跳数={}", client, status, hops);
        
        // 更新跳数并转发给客户端
        let mut forward_data = [0u8; 8];
        forward_data.copy_from_slice(&packet.data[0..8]);
        forward_data[7] = hops + 1; // 增加跳数
        
        // 创建转发给客户端的确认数据包
        let node_id = hardware.get_node_id();
        let confirm_packet = DataPacket::new(
            node_id,
            client,
            packet.header.packet_id,
            &forward_data
        );
        
        // 发送确认
        let radio = hardware.get_radio();
        if let Err(e) = radio.send_data(&confirm_packet) {
            println!("转发路径确认给客户端失败: {:?}", e);
        } else {
            println!("已转发路径确认给客户端 {:?}", client);
        }
    }
}

/// 处理其他类型的数据包
fn handle_other_packet<H: Hardware>(
    hardware: &mut H,
    forwarding_engine: &mut ForwardingEngine,
    packet: &DataPacket
) {
    let source = NodeId(packet.header.source);
    let destination = NodeId(packet.header.destination);
    
    println!("接收到来自 {:?} 发往 {:?} 的其他类型数据包，类型: {:?}",
        source, destination, packet.header.packet_type);
    
    // 如果不是发给本节点的，尝试转发
    if destination != hardware.get_node_id() && !destination.is_broadcast() {
        if let Some(next_hop) = forwarding_engine.get_next_hop(destination) {
            // 创建新的数据包进行转发
            let node_id = hardware.get_node_id();
            let forward_packet = DataPacket::new(
                node_id,
                next_hop,
                packet.header.packet_id,
                packet.data
            );
            
            // 发送转发的数据包
            let radio = hardware.get_radio();
            if let Err(e) = radio.send_data(&forward_packet) {
                println!("转发数据包失败: {:?}", e);
            }
        }
    }
} 