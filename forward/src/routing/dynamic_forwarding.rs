use core::fmt;
use common::protocol::NodeId;
use crate::routing::RoutingTable;

/// 路由表项
#[derive(Clone, Copy)]
struct RouteEntry {
    /// 目的地节点ID
    destination: NodeId,
    /// 下一跳节点ID
    next_hop: NodeId,
    /// 路由度量（这里使用信号强度）
    metric: i8,
    /// 路由生命期时间戳
    timestamp: u64,
}

impl fmt::Debug for RouteEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteEntry")
            .field("destination", &self.destination)
            .field("next_hop", &self.next_hop)
            .field("metric", &self.metric)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

/// 转发引擎，实现动态路由
pub struct ForwardingEngine {
    /// 本节点ID
    node_id: NodeId,
    /// 路由表
    routes: [Option<RouteEntry>; 32],
    /// 当前路由数
    route_count: usize,
    /// 内部计时器，用于清理过期路由
    cleanup_timer: u64,
}

impl ForwardingEngine {
    /// 创建新的转发引擎实例
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            routes: [None; 32],
            route_count: 0,
            cleanup_timer: 0,
        }
    }
    
    /// 周期性清理过期路由
    pub fn cleanup(&mut self, current_time: u64) {
        const ROUTE_EXPIRY_MS: u64 = 300_000; // 5分钟
        
        for entry in self.routes.iter_mut() {
            if let Some(route) = entry {
                if current_time - route.timestamp > ROUTE_EXPIRY_MS {
                    *entry = None;
                    self.route_count -= 1;
                }
            }
        }
    }
    
    /// 寻找空闲的路由表项
    fn find_free_slot(&self) -> Option<usize> {
        self.routes.iter().position(|entry| entry.is_none())
    }
    
    /// 寻找指定目的地的路由表项
    fn find_route(&self, destination: NodeId) -> Option<usize> {
        self.routes.iter().position(|entry| {
            if let Some(route) = entry {
                route.destination == destination
            } else {
                false
            }
        })
    }
}

impl RoutingTable for ForwardingEngine {
    fn update_route(&mut self, destination: NodeId, metric: i8) {
        // 不要为自己添加路由
        if destination == self.node_id {
            return;
        }
        
        let current_time = self.cleanup_timer;
        
        // 查找是否已存在该目的地的路由
        if let Some(index) = self.find_route(destination) {
            // 更新现有路由
            if let Some(route) = &mut self.routes[index] {
                route.metric = metric;
                route.timestamp = current_time;
            }
        } else {
            // 添加新路由
            if let Some(index) = self.find_free_slot() {
                self.routes[index] = Some(RouteEntry {
                    destination,
                    next_hop: destination, // 直接路由
                    metric,
                    timestamp: current_time,
                });
                self.route_count += 1;
            } else {
                // 路由表已满，可以实现更复杂的替换策略
                // 这里简单地替换第一个条目
                self.routes[0] = Some(RouteEntry {
                    destination,
                    next_hop: destination,
                    metric,
                    timestamp: current_time,
                });
            }
        }
    }
    
    fn get_next_hop(&self, destination: NodeId) -> Option<NodeId> {
        // 查找目的地路由
        if let Some(index) = self.find_route(destination) {
            // 返回下一跳
            self.routes[index].map(|route| route.next_hop)
        } else {
            // 没有找到路由
            None
        }
    }
    
    fn remove_route(&mut self, destination: NodeId) {
        if let Some(index) = self.find_route(destination) {
            self.routes[index] = None;
            self.route_count -= 1;
        }
    }
    
    fn clear(&mut self) {
        for entry in self.routes.iter_mut() {
            *entry = None;
        }
        self.route_count = 0;
    }
    
    fn len(&self) -> usize {
        self.route_count
    }
    
    fn is_empty(&self) -> bool {
        self.route_count == 0
    }
} 