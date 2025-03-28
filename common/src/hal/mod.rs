pub mod bearpi_hi2821;
pub mod simulator;

use crate::protocol::{Beacon, DataPacket, NodeId};

/// 无线电接口抽象
pub trait RadioInterface {
    type Error;
    
    /// 发送信标
    fn send_beacon(&mut self, beacon: &Beacon) -> Result<(), Self::Error>;
    
    /// 发送数据包
    fn send_data<'a>(&mut self, packet: &DataPacket<'a>) -> Result<(), Self::Error>;
    
    /// 接收信标
    fn receive_beacon(&mut self) -> Result<Option<Beacon>, Self::Error>;
    
    /// 接收数据包
    fn receive_data<'a>(&mut self, buffer: &'a mut [u8]) -> Result<Option<DataPacket<'a>>, Self::Error>;
    
    /// 配置无线电
    fn configure(&mut self, channel: u8, power: u8) -> Result<(), Self::Error>;
    
    /// 获取当前信号强度
    fn get_rssi(&self) -> Result<i8, Self::Error>;
}

/// 硬件抽象层接口
pub trait Hardware {
    type Error;
    type Radio: RadioInterface;
    
    /// 获取本节点ID
    fn get_node_id(&self) -> NodeId;
    
    /// 获取无线电接口
    fn get_radio(&mut self) -> &mut Self::Radio;
    
    /// 获取电池电量百分比
    fn get_battery_level(&self) -> Result<u8, Self::Error>;
    
    /// 获取当前时间戳（毫秒）
    fn get_timestamp_ms(&self) -> Result<u64, Self::Error>;
    
    /// 延时指定毫秒数
    fn delay_ms(&mut self, ms: u32) -> Result<(), Self::Error>;
    
    /// 进入低功耗模式
    fn enter_low_power_mode(&mut self) -> Result<(), Self::Error>;
    
    /// 退出低功耗模式
    fn exit_low_power_mode(&mut self) -> Result<(), Self::Error>;
} 