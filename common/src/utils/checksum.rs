/// 计算CRC-16校验和
pub fn calculate_checksum(data: &[u8]) -> u16 {
    // 使用CRC-16-CCITT多项式 0x1021
    const POLY: u16 = 0x1021;
    
    let mut crc: u16 = 0xFFFF; // 初始值
    
    for byte in data {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    
    crc
}

/// 快速验证校验和，用于判断两个数据包是否相同
pub fn verify_checksum(data: &[u8], checksum: u16) -> bool {
    calculate_checksum(data) == checksum
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_checksum() {
        // 测试向量
        let data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let expected = 0x5BCA; // 预计算的CRC-16-CCITT结果
        
        let result = calculate_checksum(&data);
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_verify_checksum() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let checksum = calculate_checksum(&data);
        
        assert!(verify_checksum(&data, checksum));
        assert!(!verify_checksum(&data, checksum + 1));
    }
} 