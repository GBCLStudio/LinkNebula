pub mod election;
pub mod service_directory;

use common::protocol::NodeId;

/// 服务目录接口
pub trait ServiceDirectory {
    /// 注册服务
    fn register_service(&mut self, node_id: NodeId, service_type: ServiceType);
    
    /// 查找服务
    fn find_service(&self, service_type: ServiceType) -> Option<NodeId>;
    
    /// 移除服务
    fn remove_service(&mut self, node_id: NodeId, service_type: ServiceType);
    
    /// 获取服务数量
    fn service_count(&self) -> usize;
}

/// 服务类型枚举，现在使用common/protocol中的ServiceType
pub use common::protocol::ServiceType; 