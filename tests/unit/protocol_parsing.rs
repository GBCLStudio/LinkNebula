#[cfg(test)]
mod protocol_parsing_tests {
    use common::protocol::{NodeId, Beacon, DataPacket, PacketType};
    use common::utils::calculate_checksum;
    
    #[test]
    fn test_beacon_creation_and_parsing() {
        let node_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let battery_level = 85;
        let rssi = -70;
        
        // 创建信标
        let beacon = Beacon::new(node_id, battery_level, rssi);
        
        // 验证信标字段
        assert_eq!(beacon.packet_type, PacketType::Beacon as u8);
        assert_eq!(beacon.source, node_id.0);
        assert_eq!(beacon.battery_level, battery_level);
        assert_eq!(beacon.rssi, rssi);
        
        // 验证校验和计算是否正确
        assert!(beacon.is_valid());
        
        // 模拟解析收到的信标
        let parsed_node_id = NodeId(beacon.source);
        assert_eq!(parsed_node_id, node_id);
    }
    
    #[test]
    fn test_data_packet_creation_and_parsing() {
        let source_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let dest_id = NodeId::new([0xS1, 0xS2, 0xS3, 0xS4, 0xS5, 0xS6]);
        let packet_id = 42;
        let test_data = [0x11, 0x22, 0x33, 0x44, 0x55];
        
        // 创建数据包
        let packet = DataPacket::new(source_id, dest_id, packet_id, &test_data);
        
        // 验证数据包字段
        assert_eq!(packet.header.packet_type, PacketType::Data as u8);
        assert_eq!(packet.header.source, source_id.0);
        assert_eq!(packet.header.destination, dest_id.0);
        assert_eq!(packet.header.packet_id, packet_id);
        assert_eq!(packet.header.data_length, test_data.len() as u16);
        assert_eq!(packet.data, test_data);
        
        // 验证校验和计算是否正确
        assert!(packet.is_valid());
        
        // 验证修改数据后校验和不再有效
        let mut test_buffer = Vec::new();
        test_buffer.extend_from_slice(&packet.header.source);
        test_buffer.extend_from_slice(&packet.header.destination);
        test_buffer.extend_from_slice(&packet.header.packet_id.to_be_bytes());
        
        // 手动计算校验和
        let checksum = calculate_checksum(&test_buffer);
        assert_ne!(checksum, packet.header.checksum); // 应该不相等，因为计算方式不同
    }
    
    #[test]
    fn test_node_id_functions() {
        let node_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let broadcast_id = NodeId::BROADCAST;
        
        // 验证广播ID
        assert!(broadcast_id.is_broadcast());
        assert!(!node_id.is_broadcast());
        
        // 验证相等性
        let same_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let different_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x07]);
        
        assert_eq!(node_id, same_id);
        assert_ne!(node_id, different_id);
    }
} 