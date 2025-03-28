pub struct StorageEngine {
    dma_channel: DmaChannel,
    buffer: AlignedBuffer<[u8; 4096]>,
}

impl StorageEngine {
    /// DMA零拷贝写入
    pub fn store_packet(&mut self, packet: &NetworkPacket) {
        // 配置DMA源地址
        let src_ptr = packet.as_bytes().as_ptr() as u32;
        
        // 获取当前写入位置
        let offset = self.next_offset();
        
        unsafe {
            // 启动DMA传输
            self.dma_channel.configure(
                src_ptr,
                self.buffer.as_ptr() as u32 + offset,
                packet.as_bytes().len() as u32,
                || {
                    // 传输完成回调
                    self.update_index();
                }
            );
            self.dma_channel.enable();
        }
    }
}