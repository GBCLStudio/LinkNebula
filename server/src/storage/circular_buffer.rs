use common::protocol::NodeId;
use crate::storage::{SensorRecord, Storage};

/// 环形缓冲区，用于存储传感器数据
pub struct CircularBuffer {
    /// 存储区
    records: [Option<SensorRecord>; 1024],
    /// 当前写入位置
    write_position: usize,
    /// 当前存储的记录数
    record_count: usize,
    /// 全局时间戳，用于给记录分配时间戳
    timestamp: u64,
}

impl CircularBuffer {
    /// 创建新的环形缓冲区
    pub fn new() -> Self {
        Self {
            records: [None; 1024],
            write_position: 0,
            record_count: 0,
            timestamp: 0,
        }
    }
    
    /// 更新内部时间戳
    pub fn update_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }
    
    /// 添加传感器记录
    fn add_record(&mut self, record: SensorRecord) {
        // 更新记录数
        if self.records[self.write_position].is_none() {
            self.record_count += 1;
        }
        
        // 写入记录
        self.records[self.write_position] = Some(record);
        
        // 更新写入位置
        self.write_position = (self.write_position + 1) % self.records.len();
    }
    
    /// 查找指定节点的所有记录
    fn find_records_for_node(&self, node_id: NodeId) -> Vec<SensorRecord> {
        let mut result = Vec::new();
        
        for record_option in self.records.iter() {
            if let Some(record) = record_option {
                if record.node_id == node_id {
                    result.push(*record);
                }
            }
        }
        
        result
    }
    
    /// 查找特定时间范围内的记录
    fn find_records_in_timerange(&self, start_time: u64, end_time: u64) -> Vec<SensorRecord> {
        let mut result = Vec::new();
        
        for record_option in self.records.iter() {
            if let Some(record) = record_option {
                if record.timestamp >= start_time && record.timestamp <= end_time {
                    result.push(*record);
                }
            }
        }
        
        result
    }
    
    /// 序列化传感器记录
    fn serialize_records(&self, records: &[SensorRecord]) -> Vec<u8> {
        let mut result = Vec::with_capacity(records.len() * 20);
        
        for record in records {
            // 记录格式：
            // 节点ID (6字节)
            // 时间戳 (8字节)
            // 温度 (2字节，定点数，乘以100)
            // 湿度 (2字节，定点数，乘以100)
            // 气压 (2字节，百帕单位)
            
            // 添加节点ID
            result.extend_from_slice(&record.node_id.0);
            
            // 添加时间戳
            result.extend_from_slice(&record.timestamp.to_be_bytes());
            
            // 添加温度
            let temp = (record.temperature * 100.0) as u16;
            result.extend_from_slice(&temp.to_be_bytes());
            
            // 添加湿度
            let humidity = (record.humidity * 100.0) as u16;
            result.extend_from_slice(&humidity.to_be_bytes());
            
            // 添加气压
            let pressure = (record.pressure / 100.0) as u16; // 转换为百帕
            result.extend_from_slice(&pressure.to_be_bytes());
        }
        
        result
    }
}

impl Storage for CircularBuffer {
    fn add_data(&mut self, node_id: NodeId, temperature: f32, humidity: f32, pressure: f32) {
        // 创建传感器记录
        let record = SensorRecord {
            node_id,
            timestamp: self.timestamp,
            temperature,
            humidity,
            pressure,
        };
        
        // 添加记录
        self.add_record(record);
        
        // 更新时间戳，这里简单地加1秒
        self.timestamp += 1000;
    }
    
    fn get_data_for_node(&self, node_id: NodeId) -> Vec<u8> {
        // 查找记录
        let records = self.find_records_for_node(node_id);
        
        // 序列化记录
        self.serialize_records(&records)
    }
    
    fn get_data_in_timerange(&self, start_time: u64, end_time: u64) -> Vec<u8> {
        // 查找记录
        let records = self.find_records_in_timerange(start_time, end_time);
        
        // 序列化记录
        self.serialize_records(&records)
    }
    
    fn clear_data_for_node(&mut self, node_id: NodeId) {
        for record in self.records.iter_mut() {
            if let Some(r) = record {
                if r.node_id == node_id {
                    *record = None;
                    self.record_count -= 1;
                }
            }
        }
    }
    
    fn clear_all_data(&mut self) {
        for record in self.records.iter_mut() {
            *record = None;
        }
        self.record_count = 0;
        self.write_position = 0;
    }
} 