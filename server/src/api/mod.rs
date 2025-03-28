pub mod cli;

use common::protocol::NodeId;
use crate::storage::Storage;

/// 命令类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    /// 查询传感器数据
    Query = 0x01,
    /// 配置采集间隔
    Configure = 0x02,
    /// 清空数据
    Clear = 0x03,
    /// 重启设备
    Reboot = 0x04,
}

/// 命令结构
#[derive(Debug)]
pub struct Command {
    /// 源节点ID
    pub source: NodeId,
    /// 命令类型
    pub command_type: CommandType,
    /// 命令参数
    pub parameters: Vec<u8>,
}

/// 命令处理接口
pub trait CommandHandler {
    /// 添加命令到队列
    fn add_command(&mut self, source: NodeId, data: &[u8]);
    
    /// 处理所有待处理的命令
    fn process_commands<H, S>(&mut self, hardware: &mut H, storage: &mut S)
    where
        H: common::hal::Hardware,
        S: Storage;
} 