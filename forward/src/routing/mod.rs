pub struct ForwardingEngine {
    routing_table: RoutingTable,
    rx_buf: AlignedBuffer<NetworkPacket>,
    tx_buf: AlignedBuffer<NetworkPacket>,
}

impl ForwardingEngine {
    pub fn process(&mut self, hal: &mut impl HalInterface) {
        // 零拷贝接收
        let len = match hal.recv(self.rx_buf.as_bytes_mut()) {
            Ok(l) => l,
            Err(_) => return,
        };
        
        let packet = self.rx_buf.get();
        if packet.header.ttl == 0 || !validate_checksum(packet) {
            return;
        }

        // 更新TTL并重新计算校验和
        let mut tx_packet = self.tx_buf.get_mut();
        *tx_packet = *packet;
        tx_packet.header.ttl -= 1;
        tx_packet.header.checksum = 0;
        tx_packet.header.checksum = crc32(tx_packet.as_bytes());

        // 查询路由表
        let next_hop = self.routing_table.lookup(packet.header.dest_mac);
        
        // 转发数据包
        hal.send(&next_hop, tx_packet.as_bytes());
    }
}