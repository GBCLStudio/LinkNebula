#[cfg(test)]
mod service_discovery_tests {
    use common::protocol::{NodeId, ServiceType, QosRequirements, DataPacket};
    use common::hal::simulator::{SimChannel, SimHardware};
    use common::protocol::{ServiceRequest, serialize_service_request, deserialize_service_response};
    use common::protocol::{PacketType, PathStatus};
    use forward::directory::service_directory::{NetworkServiceDirectory, Capabilities, ServiceMetrics};
    
    #[test]
    fn test_service_discovery_and_path_establishment() {
        // 创建共享通信信道
        let channel = SimChannel::new();
        
        // 创建客户端、转发节点和服务器节点
        let client_id = NodeId::new([0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6]);
        let forward_id = NodeId::new([0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6]);
        let server_id = NodeId::new([0xS1, 0xS2, 0xS3, 0xS4, 0xS5, 0xS6]);
        
        let mut client = SimHardware::new(client_id, channel.clone());
        let mut forward = SimHardware::new(forward_id, channel.clone());
        let mut server = SimHardware::new(server_id, channel.clone());
        
        // 1. 初始化服务目录，并注册服务器的视频中继服务
        let mut service_directory = NetworkServiceDirectory::new();
        
        let capabilities = Capabilities {
            max_bandwidth: 1000, // 1 Mbps
            min_latency: 50,     // 50ms
            reliability: 95,      // 95%可靠性
            battery_level: 80,    // 80%电池电量
        };
        
        let metrics = ServiceMetrics {
            success_rate: 100,      // 100%成功率
            avg_response_time: 20,  // 20ms平均响应时间
            signal_strength: -60,   // -60dBm信号强度
        };
        
        service_directory.update_service(
            server_id,
            ServiceType::VideoRelay,
            20, // 20%负载
            capabilities,
            metrics,
            0  // 时间戳
        );
        
        // 2. 客户端发送服务请求
        let qos = QosRequirements {
            min_bandwidth: 500, // 500kbps
            max_latency: 100,   // 100ms延迟
            reliability: 80,    // 80%可靠性
        };
        
        let service_request = ServiceRequest {
            service_type: ServiceType::VideoRelay,
            qos,
            expiry_time: 60, // 60秒
        };
        
        // 序列化请求
        let mut request_buffer = [0u8; 32];
        let request_len = serialize_service_request(&service_request, &mut request_buffer);
        
        assert!(request_len > 0, "服务请求序列化失败");
        
        // 创建请求数据包
        let request_packet = DataPacket::new(
            client_id,
            forward_id,
            1, // 包ID
            &request_buffer[..request_len]
        );
        
        // 发送请求
        client.get_radio().send_data(&request_packet).unwrap();
        
        // 3. 转发节点接收请求并处理
        let mut rx_buffer = [0u8; 256];
        let received_packet = forward.get_radio().receive_data(&mut rx_buffer).unwrap().unwrap();
        
        assert_eq!(received_packet.header.source, client_id.0);
        assert_eq!(received_packet.header.destination, forward_id.0);
        
        // 4. 查询服务目录找到最佳服务提供者
        let best_service = service_directory.find_best_service(
            ServiceType::VideoRelay, 
            &qos
        ).unwrap();
        
        assert_eq!(best_service.node_id, server_id);
        
        // 5. 转发节点向客户端发送服务响应
        let mut response_buffer = [0u8; 32];
        
        // 构造服务响应数据
        response_buffer[0] = 0x00; // 成功状态
        response_buffer[1] = 0x00;
        response_buffer[2] = 0x00;
        response_buffer[3] = 0x01; // 服务ID = 1
        response_buffer[4..10].copy_from_slice(&server_id.0); // 服务器ID
        
        // 创建响应数据包
        let response_packet = DataPacket::new(
            forward_id,
            client_id,
            1, // 包ID
            &response_buffer[..11]
        );
        
        // 发送响应
        forward.get_radio().send_data(&response_packet).unwrap();
        
        // 6. 转发节点向服务器发送路径建立请求
        let mut path_buffer = [0u8; 32];
        
        // 填充路径建立请求
        path_buffer[0..6].copy_from_slice(&client_id.0); // 客户端ID
        path_buffer[6] = ServiceType::VideoRelay as u8;  // 服务类型
        
        // 设置QoS参数
        let bandwidth_bytes = qos.min_bandwidth.to_be_bytes();
        path_buffer[7] = bandwidth_bytes[0];
        path_buffer[8] = bandwidth_bytes[1];
        
        let latency_bytes = qos.max_latency.to_be_bytes();
        path_buffer[9] = latency_bytes[0];
        path_buffer[10] = latency_bytes[1];
        
        path_buffer[11] = qos.reliability;
        
        // 创建路径建立数据包
        let path_packet = DataPacket::new(
            forward_id,
            server_id,
            2, // 新包ID
            &path_buffer[..12]
        );
        
        // 发送路径建立请求
        forward.get_radio().send_data(&path_packet).unwrap();
        
        // 7. 服务器接收路径建立请求
        let received_path = server.get_radio().receive_data(&mut rx_buffer).unwrap().unwrap();
        
        assert_eq!(received_path.header.source, forward_id.0);
        assert_eq!(received_path.header.destination, server_id.0);
        
        // 8. 服务器向转发节点发送路径确认
        let mut confirm_buffer = [0u8; 32];
        
        // 填充路径确认
        confirm_buffer[0..6].copy_from_slice(&client_id.0); // 客户端ID
        confirm_buffer[6] = PathStatus::Success as u8;      // 成功状态
        confirm_buffer[7] = 1; // 跳数为1
        
        // 创建路径确认数据包
        let confirm_packet = DataPacket::new(
            server_id,
            forward_id,
            2, // 与请求相同的包ID
            &confirm_buffer[..8]
        );
        
        // 发送路径确认
        server.get_radio().send_data(&confirm_packet).unwrap();
        
        // 9. 转发节点接收路径确认并转发给客户端
        let received_confirm = forward.get_radio().receive_data(&mut rx_buffer).unwrap().unwrap();
        
        assert_eq!(received_confirm.header.source, server_id.0);
        assert_eq!(received_confirm.header.destination, forward_id.0);
        
        // 10. 转发节点更新跳数并转发给客户端
        let mut fwd_confirm_buffer = [0u8; 32];
        fwd_confirm_buffer.copy_from_slice(&confirm_buffer[..8]);
        fwd_confirm_buffer[7] = 2; // 增加跳数为2
        
        // 创建转发给客户端的确认数据包
        let fwd_confirm_packet = DataPacket::new(
            forward_id,
            client_id,
            2, // 与请求相同的包ID
            &fwd_confirm_buffer[..8]
        );
        
        // 发送确认
        forward.get_radio().send_data(&fwd_confirm_packet).unwrap();
        
        // 11. 客户端接收路径确认
        let client_confirm = client.get_radio().receive_data(&mut rx_buffer).unwrap().unwrap();
        
        assert_eq!(client_confirm.header.source, forward_id.0);
        assert_eq!(client_confirm.header.destination, client_id.0);
        assert_eq!(client_confirm.data[6], PathStatus::Success as u8); // 确认成功状态
        assert_eq!(client_confirm.data[7], 2); // 确认跳数为2
        
        // 总结: 验证了服务发现和路径建立的完整流程
        println!("服务发现和路径建立测试通过!");
    }
} 