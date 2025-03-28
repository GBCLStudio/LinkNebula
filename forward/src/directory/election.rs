use common::protocol::{NodeId, DataPacket};
use common::hal::Hardware;
use common::utils::AlignedBuffer;
use crate::directory::ServiceType;

/// 选举协议消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElectionMessageType {
    /// 开始选举
    ElectionStart = 0x01,
    /// 竞选回应
    ElectionResponse = 0x02,
    /// 选举结果广播
    ElectionResult = 0x03,
}

/// 主服务器选举协议实现
pub struct ElectionProtocol {
    /// 本节点ID
    node_id: NodeId,
    /// 当前选举ID
    election_id: u16,
    /// 当前选举状态
    state: ElectionState,
    /// 当前主服务器
    current_master: Option<NodeId>,
    /// 接收缓冲区
    buffer: AlignedBuffer<256>,
}

/// 选举状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElectionState {
    /// 空闲
    Idle,
    /// 正在选举中
    Electing,
    /// 已完成选举
    Completed,
}

impl ElectionProtocol {
    /// 创建新的选举协议实例
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            election_id: 0,
            state: ElectionState::Idle,
            current_master: None,
            buffer: AlignedBuffer::new(),
        }
    }
    
    /// 发起选举
    pub fn initiate_election<H: Hardware>(&mut self, hardware: &mut H) {
        println!("发起主服务器选举");
        
        // 增加选举ID
        self.election_id = self.election_id.wrapping_add(1);
        self.state = ElectionState::Electing;
        
        // 创建选举消息
        let mut election_msg = [0u8; 4];
        election_msg[0] = ElectionMessageType::ElectionStart as u8;
        election_msg[1] = (self.election_id >> 8) as u8;
        election_msg[2] = (self.election_id & 0xFF) as u8;
        election_msg[3] = self.get_priority();
        
        // 广播选举消息
        let packet = DataPacket::new(
            self.node_id,
            NodeId::BROADCAST,
            self.election_id,
            &election_msg
        );
        
        let radio = hardware.get_radio();
        if let Err(e) = radio.send_data(&packet) {
            println!("发送选举消息失败: {:?}", e);
        }
        
        // 等待一段时间收集响应
        let _ = hardware.delay_ms(5000);
        
        // 结束选举并广播结果
        self.finish_election(hardware);
    }
    
    /// 结束选举并广播结果
    fn finish_election<H: Hardware>(&mut self, hardware: &mut H) {
        // 这里应该根据收集到的响应确定最佳主服务器
        // 简化实现：假设自己是主服务器
        self.current_master = Some(self.node_id);
        self.state = ElectionState::Completed;
        
        // 广播选举结果
        let mut result_msg = [0u8; 10];
        result_msg[0] = ElectionMessageType::ElectionResult as u8;
        result_msg[1] = (self.election_id >> 8) as u8;
        result_msg[2] = (self.election_id & 0xFF) as u8;
        
        // 复制主服务器节点ID
        if let Some(master) = self.current_master {
            result_msg[3..9].copy_from_slice(&master.0);
        }
        
        // 广播结果
        let packet = DataPacket::new(
            self.node_id,
            NodeId::BROADCAST,
            self.election_id,
            &result_msg
        );
        
        let radio = hardware.get_radio();
        if let Err(e) = radio.send_data(&packet) {
            println!("发送选举结果失败: {:?}", e);
        } else {
            println!("选举完成，主服务器: {:?}", self.current_master);
        }
    }
    
    /// 处理选举消息
    pub fn process_messages<H: Hardware>(&mut self, hardware: &mut H) {
        let radio = hardware.get_radio();
        let buffer = self.buffer.as_mut_slice();
        
        if let Ok(Some(packet)) = radio.receive_data(buffer) {
            // 确保数据包至少有一个字节
            if packet.data.is_empty() {
                return;
            }
            
            match packet.data[0] {
                x if x == ElectionMessageType::ElectionStart as u8 => {
                    self.handle_election_start(hardware, &packet);
                },
                x if x == ElectionMessageType::ElectionResponse as u8 => {
                    self.handle_election_response(hardware, &packet);
                },
                x if x == ElectionMessageType::ElectionResult as u8 => {
                    self.handle_election_result(hardware, &packet);
                },
                _ => {
                    // 忽略未知消息类型
                }
            }
        }
    }
    
    /// 处理选举启动消息
    fn handle_election_start<H: Hardware>(&mut self, hardware: &mut H, packet: &DataPacket) {
        if packet.data.len() < 4 {
            return; // 消息格式错误
        }
        
        // 提取选举ID
        let election_id = ((packet.data[1] as u16) << 8) | (packet.data[2] as u16);
        let sender_priority = packet.data[3];
        let source = NodeId(packet.header.source);
        
        println!("收到来自 {:?} 的选举消息，选举ID: {}", source, election_id);
        
        // 如果发送方优先级高于自己，只发送响应
        if sender_priority > self.get_priority() {
            // 发送选举响应
            let mut response = [0u8; 4];
            response[0] = ElectionMessageType::ElectionResponse as u8;
            response[1] = packet.data[1]; // 选举ID高字节
            response[2] = packet.data[2]; // 选举ID低字节
            response[3] = self.get_priority();
            
            let response_packet = DataPacket::new(
                self.node_id,
                source,
                election_id,
                &response
            );
            
            let radio = hardware.get_radio();
            if let Err(e) = radio.send_data(&response_packet) {
                println!("发送选举响应失败: {:?}", e);
            }
        } else {
            // 如果自己优先级更高，发起新一轮选举
            if self.state != ElectionState::Electing {
                self.initiate_election(hardware);
            }
        }
    }
    
    /// 处理选举响应消息
    fn handle_election_response<H: Hardware>(&mut self, hardware: &mut H, packet: &DataPacket) {
        if packet.data.len() < 4 || self.state != ElectionState::Electing {
            return; // 消息格式错误或当前不在选举状态
        }
        
        // 提取选举ID
        let election_id = ((packet.data[1] as u16) << 8) | (packet.data[2] as u16);
        
        // 检查是否是当前选举
        if election_id != self.election_id {
            return;
        }
        
        // 实际实现中，这里应该记录所有响应，用于后续确定最佳主服务器
        println!("收到来自 {:?} 的选举响应", NodeId(packet.header.source));
    }
    
    /// 处理选举结果消息
    fn handle_election_result<H: Hardware>(&mut self, hardware: &mut H, packet: &DataPacket) {
        if packet.data.len() < 9 {
            return; // 消息格式错误
        }
        
        // 提取选举ID和主服务器ID
        let election_id = ((packet.data[1] as u16) << 8) | (packet.data[2] as u16);
        let master_id = NodeId([
            packet.data[3], packet.data[4], packet.data[5],
            packet.data[6], packet.data[7], packet.data[8]
        ]);
        
        println!("收到选举结果，主服务器为: {:?}", master_id);
        
        // 更新主服务器
        self.current_master = Some(master_id);
        self.state = ElectionState::Completed;
    }
    
    /// 获取本节点优先级
    fn get_priority(&self) -> u8 {
        // 简化实现：使用节点ID的第一个字节作为优先级
        self.node_id.0[0]
    }
    
    /// 获取当前主服务器
    pub fn get_master(&self) -> Option<NodeId> {
        self.current_master
    }
} 