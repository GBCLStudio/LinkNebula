use common::protocol::{DataPacket, NodeId};
use common::hal::Hardware;
use crate::api::{Command, CommandHandler, CommandType};
use crate::storage::Storage;

/// 命令处理器
pub struct CommandProcessor {
    /// 本节点ID
    node_id: NodeId,
    /// 命令队列
    commands: [Option<Command>; 16],
    /// 写入位置
    write_position: usize,
    /// 读取位置
    read_position: usize,
}

impl CommandProcessor {
    /// 创建新的命令处理器
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            commands: [None; 16],
            write_position: 0,
            read_position: 0,
        }
    }
    
    /// 检查队列是否为空
    fn is_empty(&self) -> bool {
        self.write_position == self.read_position
    }
    
    /// 检查队列是否已满
    fn is_full(&self) -> bool {
        (self.write_position + 1) % self.commands.len() == self.read_position
    }
    
    /// 将命令数据解析为命令结构
    fn parse_command(&self, source: NodeId, data: &[u8]) -> Option<Command> {
        if data.is_empty() {
            return None;
        }
        
        // 获取命令类型
        let command_type = match data[0] {
            0x01 => CommandType::Query,
            0x02 => CommandType::Configure,
            0x03 => CommandType::Clear,
            0x04 => CommandType::Reboot,
            _ => return None, // 未知命令
        };
        
        // 获取命令参数
        let parameters = if data.len() > 1 {
            data[1..].to_vec()
        } else {
            Vec::new()
        };
        
        Some(Command {
            source,
            command_type,
            parameters,
        })
    }
    
    /// 执行查询命令
    fn execute_query<H: Hardware, S: Storage>(
        &self,
        hardware: &mut H,
        storage: &mut S,
        command: &Command
    ) {
        println!("执行查询命令");
        
        // 获取节点数据
        let data = storage.get_data_for_node(command.source);
        
        // 发送响应
        self.send_response(hardware, command.source, CommandType::Query, &data);
    }
    
    /// 执行配置命令
    fn execute_configure<H: Hardware, S: Storage>(
        &self,
        hardware: &mut H,
        storage: &mut S,
        command: &Command
    ) {
        println!("执行配置命令");
        
        // 实际中应该根据参数配置采集间隔等参数
        // 这里简单地发送确认响应
        let response = [0x01]; // 简单的确认码
        
        // 发送响应
        self.send_response(hardware, command.source, CommandType::Configure, &response);
    }
    
    /// 执行清空数据命令
    fn execute_clear<H: Hardware, S: Storage>(
        &self,
        hardware: &mut H,
        storage: &mut S,
        command: &Command
    ) {
        println!("执行清空数据命令");
        
        // 清空指定节点的数据
        storage.clear_data_for_node(command.source);
        
        // 发送确认响应
        let response = [0x01]; // 简单的确认码
        
        // 发送响应
        self.send_response(hardware, command.source, CommandType::Clear, &response);
    }
    
    /// 执行重启命令
    fn execute_reboot<H: Hardware, S: Storage>(
        &self,
        hardware: &mut H,
        storage: &mut S,
        command: &Command
    ) {
        println!("执行重启命令（模拟）");
        
        // 此处实际实现中应该真正重启设备
        // 在模拟中，只是发送确认响应
        let response = [0x01]; // 简单的确认码
        
        // 发送响应
        self.send_response(hardware, command.source, CommandType::Reboot, &response);
    }
    
    /// 发送响应
    fn send_response<H: Hardware>(
        &self,
        hardware: &mut H,
        destination: NodeId,
        command_type: CommandType,
        data: &[u8]
    ) {
        // 创建响应数据
        let mut response_data = Vec::with_capacity(data.len() + 1);
        response_data.push(command_type as u8);
        response_data.extend_from_slice(data);
        
        // 创建数据包
        let packet = DataPacket::new(
            self.node_id,
            destination,
            0, // 响应ID
            &response_data
        );
        
        // 发送数据包
        let radio = hardware.get_radio();
        if let Err(e) = radio.send_data(&packet) {
            println!("发送响应失败: {:?}", e);
        } else {
            println!("响应已发送给 {:?}", destination);
        }
    }
}

impl CommandHandler for CommandProcessor {
    fn add_command(&mut self, source: NodeId, data: &[u8]) {
        if self.is_full() {
            println!("命令队列已满，忽略新命令");
            return;
        }
        
        if let Some(command) = self.parse_command(source, data) {
            self.commands[self.write_position] = Some(command);
            self.write_position = (self.write_position + 1) % self.commands.len();
            println!("添加新命令到队列，类型: {:?}", command.command_type);
        }
    }
    
    fn process_commands<H, S>(&mut self, hardware: &mut H, storage: &mut S)
    where
        H: Hardware,
        S: Storage,
    {
        while !self.is_empty() {
            if let Some(command) = &self.commands[self.read_position] {
                match command.command_type {
                    CommandType::Query => self.execute_query(hardware, storage, command),
                    CommandType::Configure => self.execute_configure(hardware, storage, command),
                    CommandType::Clear => self.execute_clear(hardware, storage, command),
                    CommandType::Reboot => self.execute_reboot(hardware, storage, command),
                }
            }
            
            // 移除已处理的命令
            self.commands[self.read_position] = None;
            self.read_position = (self.read_position + 1) % self.commands.len();
        }
    }
} 