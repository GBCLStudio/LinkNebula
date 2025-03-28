use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use crate::hal::{Hardware, RadioInterface};
use crate::protocol::{Beacon, DataPacket, NodeId};

/// 模拟器错误类型
#[derive(Debug)]
pub enum SimulatorError {
    RadioError,
    TimerError,
    ConfigError,
}

/// 共享通信通道，用于在多个模拟节点之间传递消息
#[derive(Clone)]
pub struct SimChannel {
    beacons: Arc<Mutex<VecDeque<(NodeId, Beacon)>>>,
    packets: Arc<Mutex<VecDeque<(NodeId, Vec<u8>, usize)>>>,
}

impl SimChannel {
    pub fn new() -> Self {
        Self {
            beacons: Arc::new(Mutex::new(VecDeque::new())),
            packets: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    
    pub fn push_beacon(&self, source: NodeId, beacon: Beacon) {
        if let Ok(mut beacons) = self.beacons.lock() {
            beacons.push_back((source, beacon));
        }
    }
    
    pub fn push_packet(&self, source: NodeId, data: &[u8], len: usize) {
        if let Ok(mut packets) = self.packets.lock() {
            packets.push_back((source, data.to_vec(), len));
        }
    }
    
    pub fn get_beacon(&self, dest: NodeId) -> Option<Beacon> {
        if let Ok(mut beacons) = self.beacons.lock() {
            // 找到第一个目标为广播或特定目标的信标
            for i in 0..beacons.len() {
                let (src, beacon) = &beacons[i];
                // 忽略自己发送的信标
                if *src != dest {
                    let b = *beacon;
                    beacons.remove(i);
                    return Some(b);
                }
            }
        }
        None
    }
    
    pub fn get_packet(&self, dest: NodeId, buffer: &mut [u8]) -> Option<usize> {
        if let Ok(mut packets) = self.packets.lock() {
            // 找到第一个目标为广播或特定目标的数据包
            for i in 0..packets.len() {
                let (src, data, len) = &packets[i];
                // 忽略自己发送的数据包
                if *src != dest && *len <= buffer.len() {
                    buffer[..*len].copy_from_slice(&data[..*len]);
                    let len_copy = *len;
                    packets.remove(i);
                    return Some(len_copy);
                }
            }
        }
        None
    }
}

/// 模拟无线电接口
pub struct SimRadio {
    channel: u8,
    power: u8,
    sim_channel: SimChannel,
    node_id: NodeId,
}

impl SimRadio {
    pub fn new(sim_channel: SimChannel, node_id: NodeId) -> Self {
        Self {
            channel: 11,
            power: 20,
            sim_channel,
            node_id,
        }
    }
}

impl RadioInterface for SimRadio {
    type Error = SimulatorError;
    
    fn send_beacon(&mut self, beacon: &Beacon) -> Result<(), Self::Error> {
        self.sim_channel.push_beacon(self.node_id, *beacon);
        Ok(())
    }
    
    fn send_data<'a>(&mut self, packet: &DataPacket<'a>) -> Result<(), Self::Error> {
        // 模拟发送数据，实际上是将数据放入共享通道
        let header = unsafe {
            std::slice::from_raw_parts(
                &packet.header as *const _ as *const u8,
                std::mem::size_of::<crate::protocol::data::DataHeader>(),
            )
        };
        
        let total_len = header.len() + packet.data.len();
        let mut buffer = vec![0u8; total_len];
        
        buffer[..header.len()].copy_from_slice(header);
        buffer[header.len()..].copy_from_slice(packet.data);
        
        self.sim_channel.push_packet(self.node_id, &buffer, total_len);
        Ok(())
    }
    
    fn receive_beacon(&mut self) -> Result<Option<Beacon>, Self::Error> {
        let beacon = self.sim_channel.get_beacon(self.node_id);
        Ok(beacon)
    }
    
    fn receive_data<'a>(&mut self, buffer: &'a mut [u8]) -> Result<Option<DataPacket<'a>>, Self::Error> {
        if let Some(len) = self.sim_channel.get_packet(self.node_id, buffer) {
            if len < std::mem::size_of::<crate::protocol::data::DataHeader>() {
                return Ok(None);
            }
            
            let header_size = std::mem::size_of::<crate::protocol::data::DataHeader>();
            let header = unsafe {
                &*(buffer.as_ptr() as *const crate::protocol::data::DataHeader)
            };
            
            let data_len = header.data_length as usize;
            if header_size + data_len > len {
                return Ok(None);
            }
            
            let data = &buffer[header_size..header_size + data_len];
            let packet = DataPacket {
                header: *header,
                data,
            };
            
            Ok(Some(packet))
        } else {
            Ok(None)
        }
    }
    
    fn configure(&mut self, channel: u8, power: u8) -> Result<(), Self::Error> {
        if channel < 11 || channel > 26 {
            return Err(SimulatorError::ConfigError);
        }
        
        if power > 30 {
            return Err(SimulatorError::ConfigError);
        }
        
        self.channel = channel;
        self.power = power;
        Ok(())
    }
    
    fn get_rssi(&self) -> Result<i8, Self::Error> {
        // 随机模拟一个合理的RSSI值
        let rssi = -70 - (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() % 20) as i8;
        Ok(rssi)
    }
}

/// 模拟器硬件实现
pub struct SimHardware {
    node_id: NodeId,
    radio: SimRadio,
    start_time: Instant,
    battery_level: u8,
}

impl SimHardware {
    pub fn new(node_id: NodeId, sim_channel: SimChannel) -> Self {
        Self {
            node_id,
            radio: SimRadio::new(sim_channel, node_id),
            start_time: Instant::now(),
            battery_level: 100,
        }
    }
    
    // 模拟电池消耗
    pub fn simulate_battery_drain(&mut self, percent: u8) {
        if self.battery_level > percent {
            self.battery_level -= percent;
        } else {
            self.battery_level = 0;
        }
    }
}

impl Hardware for SimHardware {
    type Error = SimulatorError;
    type Radio = SimRadio;
    
    fn get_node_id(&self) -> NodeId {
        self.node_id
    }
    
    fn get_radio(&mut self) -> &mut Self::Radio {
        &mut self.radio
    }
    
    fn get_battery_level(&self) -> Result<u8, Self::Error> {
        Ok(self.battery_level)
    }
    
    fn get_timestamp_ms(&self) -> Result<u64, Self::Error> {
        let elapsed = self.start_time.elapsed();
        Ok(elapsed.as_secs() * 1000 + elapsed.subsec_millis() as u64)
    }
    
    fn delay_ms(&mut self, ms: u32) -> Result<(), Self::Error> {
        thread::sleep(Duration::from_millis(ms as u64));
        // 模拟延迟也会消耗电池
        if ms > 1000 {
            self.simulate_battery_drain(1);
        }
        Ok(())
    }
    
    fn enter_low_power_mode(&mut self) -> Result<(), Self::Error> {
        // 模拟器中仅记录一下
        println!("Node {:?} entered low power mode", self.node_id);
        Ok(())
    }
    
    fn exit_low_power_mode(&mut self) -> Result<(), Self::Error> {
        // 模拟器中仅记录一下
        println!("Node {:?} exited low power mode", self.node_id);
        Ok(())
    }
} 