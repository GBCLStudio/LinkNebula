#[cfg(test)]
mod dynamic_config_tests {
    use common::protocol::{NodeId, DataPacket};
    use common::hal::simulator::{SimChannel, SimHardware};
    use server::api::CommandType;
    
    #[test]
    fn test_dynamic_configuration() {
        let channel = SimChannel::new();
        
        // 创建客户端和服务器节点
        let client_id = NodeId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let server_id = NodeId::new([0xS1, 0xS2, 0xS3, 0xS4, 0xS5, 0xS6]);
        
        let mut client = SimHardware::new(client_id, channel.clone());
        let mut server = SimHardware::new(server_id, channel.clone());
        
        // 测试从客户端发送配置命令到服务器
        let config_data = [
            CommandType::Configure as u8, // 命令类型
            0x05, // 配置参数1：新的采集间隔（秒）
            0x01, // 配置参数2：开启高精度模式
        ];
        
        let packet = DataPacket::new(client_id, server_id, 1, &config_data);
        
        // 客户端发送配置命令数据包
        client.get_radio().send_data(&packet).unwrap();
        
        // 缓冲区用于接收数据
        let mut buffer = [0u8; 256];
        
        // 服务器接收配置命令
        if let Ok(Some(received_packet)) = server.get_radio().receive_data(&mut buffer) {
            assert_eq!(received_packet.header.source, client_id.0);
            assert_eq!(received_packet.header.destination, server_id.0);
            assert_eq!(received_packet.data[0], CommandType::Configure as u8);
            assert_eq!(received_packet.data[1], 0x05); // 验证参数1
            assert_eq!(received_packet.data[2], 0x01); // 验证参数2
            
            // 服务器应该创建一个确认响应，但这里我们不实现具体的命令处理器，
            // 只是简单地模拟服务器会返回的响应
            let response_data = [
                CommandType::Configure as u8, // 确认是配置命令响应
                0x01, // 简单确认码：成功
            ];
            
            // 创建响应数据包
            let response_packet = DataPacket::new(
                server_id,
                client_id,
                received_packet.header.packet_id,
                &response_data
            );
            
            // 服务器发送响应
            server.get_radio().send_data(&response_packet).unwrap();
        } else {
            panic!("服务器未能接收到客户端的配置命令");
        }
        
        // 客户端接收服务器的响应
        if let Ok(Some(received_packet)) = client.get_radio().receive_data(&mut buffer) {
            assert_eq!(received_packet.header.source, server_id.0);
            assert_eq!(received_packet.header.destination, client_id.0);
            assert_eq!(received_packet.data[0], CommandType::Configure as u8);
            assert_eq!(received_packet.data[1], 0x01); // 验证确认码
        } else {
            panic!("客户端未能接收到服务器的响应");
        }
    }
} 