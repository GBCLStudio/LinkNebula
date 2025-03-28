#[repr(C)]
pub struct NearlinkConfig {
    channel: u8,
    tx_power: i8,
    pan_id: u16,
}

extern "C" {
    fn nl_init(config: *const NearlinkConfig) -> i32;
    fn nl_send(dest: *const u8, data: *const u8, len: usize) -> i32;
    fn nl_recv(buf: *mut u8, max_len: usize, actual_len: *mut usize) -> i32;
    fn nl_configure(channel: u8, tx_power: i8) -> i32;
}

pub struct BearPiHal {
    config: NearlinkConfig,
    rx_buffer: [u8; 256],
    rx_len: usize,
}

impl BearPiHal {
    pub fn new(node_id: NodeId) -> Self {
        let config = NearlinkConfig {
            channel: 15,
            tx_power: 20,
            pan_id: 0x1234,
        };
        
        let mut hal = Self {
            config,
            rx_buffer: [0; 256],
            rx_len: 0,
        };
        
        // 初始化硬件
        unsafe {
            nl_init(&hal.config as *const NearlinkConfig);
        }
        
        hal
    }
    
    pub fn configure(&mut self, channel: u8, tx_power: i8) -> Result<(), HalError> {
        unsafe {
            let ret = nl_configure(channel, tx_power);
            if ret == 0 {
                self.config.channel = channel;
                self.config.tx_power = tx_power;
                Ok(())
            } else {
                Err(HalError::ConfigFailed)
            }
        }
    }
}

impl HalInterface for BearPiHal {
    fn send(&mut self, dest: &[u8; 6], data: &[u8]) -> Result<(), HalError> {
        unsafe {
            let ret = nl_send(dest.as_ptr(), data.as_ptr(), data.len());
            if ret == 0 {
                Ok(())
            } else {
                Err(HalError::SendFailed)
            }
        }
    }
    
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, HalError> {
        let mut actual_len: usize = 0;
        
        unsafe {
            let ret = nl_recv(buf.as_mut_ptr(), buf.len(), &mut actual_len as *mut usize);
            
            if ret == 0 {
                Ok(actual_len)
            } else if ret == -1 {
                // 没有数据可接收
                Err(HalError::NoData)
            } else {
                // 其他错误
                Err(HalError::RecvFailed)
            }
        }
    }
    
    fn get_timestamp_ms(&self) -> Result<u64, HalError> {
        // 获取系统时间戳
        extern "C" {
            fn nl_get_timestamp() -> u64;
        }
        
        unsafe {
            Ok(nl_get_timestamp())
        }
    }
    
    fn delay_ms(&mut self, ms: u32) -> Result<(), HalError> {
        // 延时函数
        extern "C" {
            fn nl_delay_ms(ms: u32);
        }
        
        unsafe {
            nl_delay_ms(ms);
            Ok(())
        }
    }
}