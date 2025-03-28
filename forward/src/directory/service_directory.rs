use common::protocol::{NodeId, ServiceType, QosRequirements};
use crate::directory::ServiceDirectory;
use core::fmt;

// 服务条目
#[derive(Clone)]
pub struct ServiceEntry {
    pub node_id: NodeId,
    pub service_type: ServiceType,
    pub load: u8,                // 服务器负载 (0-100%)
    pub capabilities: Capabilities,
    pub last_update_time: u64,   // 最后更新时间戳
    pub metrics: ServiceMetrics,
}

// 服务器能力
#[derive(Clone, Copy)]
pub struct Capabilities {
    pub max_bandwidth: u16,      // 最大带宽 (kbps)
    pub min_latency: u16,        // 最小延迟 (ms)
    pub reliability: u8,         // 可靠性 (0-100%)
    pub battery_level: u8,       // 电池电量 (0-100%)
}

// 服务性能指标
#[derive(Clone, Copy)]
pub struct ServiceMetrics {
    pub success_rate: u8,        // 成功率 (0-100%)
    pub avg_response_time: u16,  // 平均响应时间 (ms)
    pub signal_strength: i8,     // 信号强度 (dBm)
}

impl fmt::Debug for ServiceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceEntry")
            .field("node_id", &self.node_id)
            .field("service_type", &self.service_type)
            .field("load", &self.load)
            .field("last_update_time", &self.last_update_time)
            .finish()
    }
}

impl ServiceEntry {
    // 评分函数 - 评估服务条目与QoS需求的匹配程度
    pub fn score(&self, qos: &QosRequirements) -> u16 {
        let mut score: u16 = 0;
        
        // 带宽评分 (高于要求的带宽给更高分)
        if self.capabilities.max_bandwidth >= qos.min_bandwidth {
            score += 40 * (1 + (self.capabilities.max_bandwidth - qos.min_bandwidth).min(1000) / 100) as u16;
        } else {
            return 0; // 不满足最低带宽要求
        }
        
        // 延迟评分 (低于要求的延迟给更高分)
        if self.capabilities.min_latency <= qos.max_latency {
            score += 30 * (1 + (qos.max_latency - self.capabilities.min_latency).min(500) / 50) as u16;
        } else {
            return 0; // 不满足最大延迟要求
        }
        
        // 可靠性评分
        if self.capabilities.reliability >= qos.reliability {
            score += 20 * (1 + (self.capabilities.reliability - qos.reliability).min(50) / 10) as u16;
        } else {
            return 0; // 不满足可靠性要求
        }
        
        // 负载评分 (负载越低越好)
        score += 10 * (100 - self.load as u16) / 10;
        
        // 电池电量评分 (电量越高越好)
        score += 5 * self.capabilities.battery_level as u16 / 10;
        
        // 信号强度评分
        let signal_factor = if self.metrics.signal_strength > -60 {
            5
        } else if self.metrics.signal_strength > -75 {
            3
        } else if self.metrics.signal_strength > -90 {
            1
        } else {
            0
        };
        score += signal_factor;
        
        score
    }
}

// 网络服务目录实现
pub struct NetworkServiceDirectory {
    services: [Option<ServiceEntry>; 32], // 最多32个服务
    service_count: usize,
    last_cleanup_time: u64,
}

impl NetworkServiceDirectory {
    // 创建新的服务目录
    pub fn new() -> Self {
        Self {
            services: [None; 32],
            service_count: 0,
            last_cleanup_time: 0,
        }
    }
    
    // 定期清理过期的服务（超过5分钟没有更新）
    pub fn cleanup(&mut self, current_time: u64) {
        const SERVICE_EXPIRY_MS: u64 = 300_000; // 5分钟
        
        // 每30秒执行一次清理
        if current_time - self.last_cleanup_time < 30_000 {
            return;
        }
        
        for entry in self.services.iter_mut() {
            if let Some(service) = entry {
                if current_time - service.last_update_time > SERVICE_EXPIRY_MS {
                    *entry = None;
                    self.service_count -= 1;
                }
            }
        }
        
        self.last_cleanup_time = current_time;
    }
    
    // 寻找指定节点和服务类型的服务
    fn find_service_index(&self, node_id: NodeId, service_type: ServiceType) -> Option<usize> {
        self.services.iter().position(|entry| {
            if let Some(service) = entry {
                service.node_id == node_id && service.service_type == service_type
            } else {
                false
            }
        })
    }
    
    // 寻找空闲的服务条目槽位
    fn find_free_slot(&self) -> Option<usize> {
        self.services.iter().position(|entry| entry.is_none())
    }
    
    // 查找最适合满足QoS需求的服务
    pub fn find_best_service(&self, service_type: ServiceType, qos: &QosRequirements) -> Option<&ServiceEntry> {
        let mut best_service: Option<&ServiceEntry> = None;
        let mut best_score: u16 = 0;
        
        for entry in self.services.iter() {
            if let Some(service) = entry {
                if service.service_type == service_type {
                    let score = service.score(qos);
                    if score > best_score {
                        best_score = score;
                        best_service = Some(service);
                    }
                }
            }
        }
        
        best_service
    }
    
    // 更新服务条目（添加新服务或更新现有服务）
    pub fn update_service(
        &mut self, 
        node_id: NodeId, 
        service_type: ServiceType,
        load: u8,
        capabilities: Capabilities,
        metrics: ServiceMetrics,
        current_time: u64
    ) -> bool {
        // 检查是否存在相同的服务条目
        if let Some(index) = self.find_service_index(node_id, service_type) {
            // 更新现有条目
            if let Some(service) = &mut self.services[index] {
                service.load = load;
                service.capabilities = capabilities;
                service.metrics = metrics;
                service.last_update_time = current_time;
            }
            return true;
        }
        
        // 添加新条目
        if let Some(index) = self.find_free_slot() {
            self.services[index] = Some(ServiceEntry {
                node_id,
                service_type,
                load,
                capabilities,
                metrics,
                last_update_time: current_time,
            });
            self.service_count += 1;
            return true;
        }
        
        // 服务目录已满
        false
    }
    
    // 获取所有与特定服务类型匹配的服务
    pub fn get_services_by_type(&self, service_type: ServiceType) -> Vec<&ServiceEntry> {
        let mut result = Vec::new();
        
        for entry in self.services.iter() {
            if let Some(service) = entry {
                if service.service_type == service_type {
                    result.push(service);
                }
            }
        }
        
        result
    }
}

impl ServiceDirectory for NetworkServiceDirectory {
    fn register_service(&mut self, node_id: NodeId, service_type: ServiceType) {
        // 简化版本，使用默认值
        let capabilities = Capabilities {
            max_bandwidth: 1000, // 1 Mbps
            min_latency: 100,    // 100ms
            reliability: 90,      // 90%
            battery_level: 100,   // 满电
        };
        
        let metrics = ServiceMetrics {
            success_rate: 100,    // 100%
            avg_response_time: 50, // 50ms
            signal_strength: -70, // -70dBm
        };
        
        self.update_service(
            node_id, 
            service_type, 
            0, // 初始负载为0
            capabilities,
            metrics,
            0 // 当前时间由调用者提供
        );
    }
    
    fn find_service(&self, service_type: ServiceType) -> Option<NodeId> {
        // 简化版本，只考虑服务类型匹配，不考虑QoS
        for entry in self.services.iter() {
            if let Some(service) = entry {
                if service.service_type == service_type {
                    return Some(service.node_id);
                }
            }
        }
        None
    }
    
    fn remove_service(&mut self, node_id: NodeId, service_type: ServiceType) {
        if let Some(index) = self.find_service_index(node_id, service_type) {
            self.services[index] = None;
            self.service_count -= 1;
        }
    }
    
    fn service_count(&self) -> usize {
        self.service_count
    }
} 