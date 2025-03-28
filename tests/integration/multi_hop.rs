#[cfg(test)]
mod multi_hop_tests {
    use common::protocol::{NodeId, DataPacket};
    use common::hal::simulator::{SimChannel, SimHardware};
    
    #[test]
    fn test_multi_hop_communication() {
        let channel = SimChannel::new();
        
        // 创建三个节点：客户端、转发节点和服务器
        let client_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let forwarder_id = NodeId::new([0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6]);
        let server_id = NodeId::new([0xS1, 0xS2, 0xS3, 0xS4, 0xS5, 0xS6]);
        
        let mut client = SimHardware::new(client_id, channel.clone());
        let mut forwarder = SimHardware::new(forwarder_id, channel.clone());
        let mut server = SimHardware::new(server_id, channel.clone());
        
        // 测试从客户端到服务器的数据包是否能通过转发节点
        let test_data = [0x01, 0x02, 0x03, 0x04];
        let packet = DataPacket::new(client_id, server_id, 1, &test_data);
        
        // 客户端发送数据包
        client.get_radio().send_data(&packet).unwrap();
        
        // 缓冲区用于接收数据
        let mut buffer = [0u8; 256];
        
        // 转发节点接收数据包
        if let Ok(Some(received_packet)) = forwarder.get_radio().receive_data(&mut buffer) {
            assert_eq!(received_packet.header.source, client_id.0);
            assert_eq!(received_packet.header.destination, server_id.0);
            assert_eq!(received_packet.data, test_data);
            
            // 转发节点创建新的数据包转发
            let forwarded_packet = DataPacket::new(
                forwarder_id,
                server_id, 
                received_packet.header.packet_id,
                received_packet.data
            );
            
            // 转发节点发送转发的数据包
            forwarder.get_radio().send_data(&forwarded_packet).unwrap();
        } else {
            panic!("转发节点未能接收到客户端的数据包");
        }
        
        // 服务器接收转发的数据包
        if let Ok(Some(received_packet)) = server.get_radio().receive_data(&mut buffer) {
            assert_eq!(received_packet.header.source, forwarder_id.0);
            assert_eq!(received_packet.header.destination, server_id.0);
            assert_eq!(received_packet.data, test_data);
        } else {
            panic!("服务器未能接收到转发节点的数据包");
        }
    }
} 