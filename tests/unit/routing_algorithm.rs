#[cfg(test)]
mod routing_algorithm_tests {
    use common::protocol::NodeId;
    use forward::routing::RoutingTable;
    use forward::routing::dynamic_forwarding::ForwardingEngine;
    
    #[test]
    fn test_routing_table_basic_operations() {
        let node_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let mut engine = ForwardingEngine::new(node_id);
        
        // 验证初始状态
        assert_eq!(engine.len(), 0);
        assert!(engine.is_empty());
        
        // 添加路由
        let destination1 = NodeId::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        let destination2 = NodeId::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        
        engine.update_route(destination1, -60);
        engine.update_route(destination2, -70);
        
        // 验证路由数量
        assert_eq!(engine.len(), 2);
        assert!(!engine.is_empty());
        
        // 验证下一跳
        let next_hop1 = engine.get_next_hop(destination1);
        let next_hop2 = engine.get_next_hop(destination2);
        
        assert!(next_hop1.is_some());
        assert!(next_hop2.is_some());
        assert_eq!(next_hop1.unwrap(), destination1);
        assert_eq!(next_hop2.unwrap(), destination2);
        
        // 删除一个路由
        engine.remove_route(destination1);
        
        assert_eq!(engine.len(), 1);
        assert!(engine.get_next_hop(destination1).is_none());
        assert!(engine.get_next_hop(destination2).is_some());
        
        // 清空路由表
        engine.clear();
        
        assert_eq!(engine.len(), 0);
        assert!(engine.is_empty());
        assert!(engine.get_next_hop(destination2).is_none());
    }
    
    #[test]
    fn test_route_update_with_better_metric() {
        let node_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let mut engine = ForwardingEngine::new(node_id);
        
        let destination = NodeId::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        
        // 添加初始路由，信号强度比较弱
        engine.update_route(destination, -80);
        
        // 使用更好的信号强度更新路由
        engine.update_route(destination, -60);
        
        // 验证路由数量仍然是1（更新而不是添加）
        assert_eq!(engine.len(), 1);
        
        // 确保路由仍然有效
        let next_hop = engine.get_next_hop(destination);
        assert!(next_hop.is_some());
        assert_eq!(next_hop.unwrap(), destination);
    }
    
    #[test]
    fn test_no_route_to_self() {
        let node_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let mut engine = ForwardingEngine::new(node_id);
        
        // 尝试添加到自己的路由
        engine.update_route(node_id, -50);
        
        // 验证没有添加路由（路由表应该为空）
        assert_eq!(engine.len(), 0);
        assert!(engine.is_empty());
        
        // 确保没有到自己的路由
        let next_hop = engine.get_next_hop(node_id);
        assert!(next_hop.is_none());
    }
} 