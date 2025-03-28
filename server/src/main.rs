#![cfg_attr(not(feature = "simulator"), no_std)]
#![cfg_attr(not(feature = "simulator"), no_main)]

mod storage;
mod api;

use common::protocol::{Beacon, DataPacket, NodeId};
use common::hal::Hardware;
use common::utils::AlignedBuffer;
use storage::circular_buffer::CircularBuffer;
use api::cli::CommandProcessor;

#[cfg(feature = "simulator")]
fn main() {
    // 模拟器入口
    use common::hal::simulator::{SimChannel, SimHardware};
    use std::thread;
    use std::time::Duration;
    
    println!("启动AetherLink服务端节点（模拟器模式）");
    
    let channel = SimChannel::new();
    let node_id = NodeId::new([0xS1, 0xS2, 0xS3, 0xS4, 0xS5, 0xS6]);
    let mut hardware = SimHardware::new(node_id, channel);
    
    server_main(&mut hardware);
}

#[cfg(feature = "bearpi")]
#[cortex_m_rt::entry]
fn main() -> ! {
    // BearPi硬件入口
    use common::hal::bearpi_hi2821::BearPiHardware;
    
    // 初始化BearPi硬件
    let node_id = NodeId::new([0xS1, 0xS2, 0xS3, 0xS4, 0xS5, 0xS6]);
    let mut hardware = BearPiHardware::new(node_id);
    
    server_main(&mut hardware);
    
    // 嵌入式设备不应该退出主循环
    loop {
        // 无限循环避免退出
    }
}

fn server_main<H: Hardware>(hardware: &mut H) {
    // 配置无线电
    let radio = hardware.get_radio();
    let _ = radio.configure(15, 20); // 使用15号信道，20dBm发射功率
    
    // 初始化存储
    let mut data_storage = CircularBuffer::new();
    
    // 初始化命令处理器
    let mut command_processor = CommandProcessor::new(hardware.get_node_id());
    
    // 创建缓冲区
    let mut rx_buffer = AlignedBuffer::<1024>::new();
    let mut beacon_timer: u64 = 0;
    
    println!("服务端节点启动完成，开始执行主循环");
    
    // 主循环
    loop {
        // 获取当前时间
        let now = hardware.get_timestamp_ms().unwrap_or(0);
        
        // 每30秒广播一次信标，让客户端能够发现服务器
        if now - beacon_timer > 30000 {
            send_beacon(hardware);
            beacon_timer = now;
        }
        
        // 接收数据包
        let radio = hardware.get_radio();
        let buffer = rx_buffer.as_mut_slice();
        
        if let Ok(Some(packet)) = radio.receive_data(buffer) {
            handle_data_packet(hardware, &mut data_storage, &mut command_processor, &packet);
        }
        
        // 处理命令
        command_processor.process_commands(hardware, &mut data_storage);
        
        // 每500毫秒做一次延迟，可以根据实际硬件调整
        let _ = hardware.delay_ms(500);
    }
}

/// 发送服务器信标
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
        println!("发送服务器信标，电池电量: {}%", battery_level);
    }
}

/// 处理接收到的数据包
fn handle_data_packet<H: Hardware>(
    hardware: &mut H,
    storage: &mut CircularBuffer,
    command_processor: &mut CommandProcessor,
    packet: &DataPacket
) {
    let source = NodeId(packet.header.source);
    
    println!("接收到来自 {:?} 的数据包，大小: {} 字节",
        source, packet.data.len());
    
    // 处理数据包类型
    if !packet.data.is_empty() {
        match packet.data[0] {
            // 传感器数据
            0x01 => {
                println!("接收到传感器数据");
                // 存储传感器数据
                if packet.data.len() >= 6 {
                    let temp = packet.data[0] as f32 + (packet.data[1] as f32) / 100.0;
                    let humidity = packet.data[2] as f32 + (packet.data[3] as f32) / 100.0;
                    let pressure = (packet.data[4] as f32) * 100.0 + (packet.data[5] as f32);
                    
                    // 存储数据
                    storage.add_data(source, temp, humidity, pressure);
                    
                    println!("存储传感器数据: 温度={}°C, 湿度={}%, 气压={}hPa",
                             temp, humidity, pressure / 100.0);
                }
            },
            // 命令
            0x02 => {
                println!("接收到命令");
                command_processor.add_command(source, &packet.data[1..]);
            },
            // 查询
            0x03 => {
                println!("接收到查询");
                // 处理查询，返回存储的数据
                let data = storage.get_data_for_node(source);
                send_response(hardware, source, &data);
            },
            _ => println!("接收到未知类型的数据包: {}", packet.data[0]),
        }
    }
}

/// 发送响应数据包
fn send_response<H: Hardware>(
    hardware: &mut H,
    destination: NodeId,
    data: &[u8]
) {
    // 创建响应数据包
    let node_id = hardware.get_node_id();
    let packet = DataPacket::new(
        node_id,
        destination,
        0, // 响应ID
        data
    );
    
    // 发送响应
    let radio = hardware.get_radio();
    if let Err(e) = radio.send_data(&packet) {
        println!("发送响应失败: {:?}", e);
    } else {
        println!("响应已发送给 {:?}", destination);
    }
} 