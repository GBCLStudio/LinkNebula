#![cfg_attr(not(feature = "simulator"), no_std)]
#![cfg_attr(not(feature = "simulator"), no_main)]

mod sensor_driver;
mod discovery;
mod service_client;

use common::protocol::{NodeId, Beacon, DataPacket, ServiceType, QosRequirements, PacketType, PathStatus};
use common::hal::Hardware;
use common::utils::AlignedBuffer;
use sensor_driver::SensorData;
use discovery::find_server;
use service_client::{request_service, ServiceEndpoint};

#[cfg(feature = "simulator")]
fn main() {
    // 模拟器入口
    use common::hal::simulator::{SimChannel, SimHardware};
    use std::thread;
    use std::time::Duration;
    
    println!("启动AetherLink客户端（模拟器模式）");
    
    let channel = SimChannel::new();
    let node_id = NodeId::new([0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6]);
    let mut hardware = SimHardware::new(node_id, channel);
    
    client_main(&mut hardware);
}

#[cfg(feature = "bearpi")]
#[cortex_m_rt::entry]
fn main() -> ! {
    // BearPi硬件入口
    use common::hal::bearpi_hi2821::BearPiHardware;
    
    // 初始化BearPi硬件
    let node_id = NodeId::new([0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6]);
    let mut hardware = BearPiHardware::new(node_id);
    
    client_main(&mut hardware);
    
    // 嵌入式设备不应该退出主循环
    loop {
        // 无限循环避免退出
    }
}

fn client_main<H: Hardware>(hardware: &mut H) {
    // 配置无线电
    let radio = hardware.get_radio();
    let _ = radio.configure(15, 20); // 使用15号信道，20dBm发射功率
    
    // 初始化传感器
    let mut sensor = sensor_driver::init_sensors().unwrap();
    
    // 创建缓冲区
    let mut rx_buffer = AlignedBuffer::<1024>::new();
    let mut tx_buffer = AlignedBuffer::<256>::new();
    
    // 发现服务器节点（转发节点）
    println!("正在搜索网络...");
    
    let mut forward_node = None;
    let mut retry_count = 0;
    
    while forward_node.is_none() && retry_count < 5 {
        forward_node = find_server(hardware);
        
        if forward_node.is_none() {
            println!("未找到转发节点，重试 {}/5", retry_count + 1);
            let _ = hardware.delay_ms(5000); // 等待5秒再尝试
            retry_count += 1;
        }
    }
    
    if forward_node.is_none() {
        println!("无法找到转发节点，退出");
        return;
    }
    
    let forward_id = forward_node.unwrap();
    println!("找到转发节点: {:?}", forward_id);
    
    // 请求视频中继服务
    let mut service_endpoint: Option<ServiceEndpoint> = None;
    
    println!("正在请求视频中继服务...");
    
    // 设置服务质量要求
    let qos = QosRequirements {
        min_bandwidth: 500, // 至少500kbps带宽
        max_latency: 200,   // 最大200ms延迟
        reliability: 80,    // 80%可靠性
    };
    
    // 请求视频中继服务
    service_endpoint = request_service(
        hardware,
        forward_id,
        ServiceType::VideoRelay,
        &qos,
        60, // 60秒过期时间
        &mut tx_buffer,
        &mut rx_buffer
    );
    
    if let Some(endpoint) = &service_endpoint {
        println!("成功获取视频中继服务：服务器={:?}, 服务ID={}", 
                 endpoint.server_id, endpoint.service_id);
    } else {
        println!("无法获取视频中继服务，退出");
        return;
    }
    
    // 等待路径建立完成
    println!("等待中继路径建立...");
    
    let mut path_established = false;
    let mut path_timer: u64 = 0;
    let mut data_send_timer: u64 = 0;
    
    // 主循环
    loop {
        // 获取当前时间
        let now = hardware.get_timestamp_ms().unwrap_or(0);
        
        // 处理收到的数据包
        let radio = hardware.get_radio();
        let buffer = rx_buffer.as_mut_slice();
        
        if let Ok(Some(packet)) = radio.receive_data(buffer) {
            match packet.header.packet_type {
                PacketType::PathConfirm => {
                    // 处理路径确认
                    if packet.data.len() >= 8 {
                        let status = packet.data[6];
                        
                        if status == PathStatus::Success as u8 {
                            path_established = true;
                            println!("中继路径建立成功，跳数: {}", packet.data[7]);
                        } else {
                            println!("中继路径建立失败，状态: {}", status);
                        }
                    }
                },
                _ => {
                    // 处理其他数据包
                    println!("收到数据包，类型: {:?}", packet.header.packet_type);
                }
            }
        }
        
        // 如果路径已建立，发送视频数据
        if path_established && service_endpoint.is_some() {
            let endpoint = service_endpoint.as_ref().unwrap();
            
            // 每500毫秒发送一次数据
            if now - data_send_timer > 500 {
                // 模拟读取视频帧数据
                let sensor_data = sensor_driver::read_sensors();
                
                // 在实际应用中，这里应该是视频数据
                // 这里为了演示，我们发送传感器数据
                send_video_data(
                    hardware,
                    endpoint,
                    &sensor_data,
                    &mut tx_buffer
                );
                
                data_send_timer = now;
            }
        } else if !path_established && now - path_timer > 30000 {
            // 等待路径建立超时（30秒）
            println!("等待路径建立超时，退出");
            return;
        }
        
        // 延迟100ms
        let _ = hardware.delay_ms(100);
    }
}

// 发送视频数据
fn send_video_data<H: Hardware>(
    hardware: &mut H,
    endpoint: &ServiceEndpoint,
    sensor_data: &SensorData, // 在实际应用中，这应该是视频帧数据
    tx_buffer: &mut AlignedBuffer<256>
) {
    // 在实际应用中，这里应该序列化视频帧数据
    // 这里为了演示，我们序列化传感器数据
    let mut data = [0u8; 32];
    
    // 0: 标识为视频数据
    data[0] = 0x01;
    
    // 1-4: 服务ID
    let service_id_bytes = endpoint.service_id.to_be_bytes();
    data[1..5].copy_from_slice(&service_id_bytes);
    
    // 5-8: 帧序号（使用当前时间作为简单的序号）
    let timestamp = hardware.get_timestamp_ms().unwrap_or(0);
    let frame_number = (timestamp % 10000) as u32;
    let frame_bytes = frame_number.to_be_bytes();
    data[5..9].copy_from_slice(&frame_bytes);
    
    // 9-12: 温度（模拟视频数据）
    let temp_bytes = sensor_data.temperature.to_be_bytes();
    data[9..13].copy_from_slice(&temp_bytes);
    
    // 13-16: 湿度（模拟视频数据）
    let humidity_bytes = sensor_data.humidity.to_be_bytes();
    data[13..17].copy_from_slice(&humidity_bytes);
    
    // 17-20: 气压（模拟视频数据）
    let pressure_bytes = sensor_data.pressure.to_be_bytes();
    data[17..21].copy_from_slice(&pressure_bytes);
    
    // 创建视频数据包
    let node_id = hardware.get_node_id();
    let packet = DataPacket::new(
        node_id,
        endpoint.server_id,
        frame_number as u16, // 使用帧号作为包ID
        &data[..21]
    );
    
    // 发送数据包
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_data(&packet) {
        println!("发送视频数据失败: {:?}", e);
    } else {
        println!("已发送视频帧 #{}", frame_number);
    }
}