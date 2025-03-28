#[cfg(not(feature = "simulator"))]
use core::mem::MaybeUninit;
#[cfg(not(feature = "simulator"))]
use core::ptr;

/// 对齐的缓冲区，用于DMA传输
#[repr(align(4))]
pub struct AlignedBuffer<const N: usize> {
    #[cfg(not(feature = "simulator"))]
    buffer: [MaybeUninit<u8>; N],
    #[cfg(feature = "simulator")]
    buffer: [u8; N],
    len: usize,
}

impl<const N: usize> AlignedBuffer<N> {
    /// 创建一个新的空缓冲区
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "simulator"))]
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            #[cfg(feature = "simulator")]
            buffer: [0; N],
            len: 0,
        }
    }
    
    /// 获取缓冲区的可变引用
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        #[cfg(not(feature = "simulator"))]
        unsafe {
            core::slice::from_raw_parts_mut(self.buffer.as_mut_ptr() as *mut u8, N)
        }
        #[cfg(feature = "simulator")]
        &mut self.buffer[..]
    }
    
    /// 获取缓冲区的只读引用
    pub fn as_slice(&self) -> &[u8] {
        #[cfg(not(feature = "simulator"))]
        unsafe {
            core::slice::from_raw_parts(self.buffer.as_ptr() as *const u8, self.len)
        }
        #[cfg(feature = "simulator")]
        &self.buffer[..self.len]
    }
    
    /// 设置有效数据长度
    pub fn set_len(&mut self, len: usize) {
        assert!(len <= N);
        self.len = len;
    }
    
    /// 获取有效数据长度
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// 判断缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.len = 0;
    }
    
    /// 复制数据到缓冲区
    pub fn copy_from_slice(&mut self, data: &[u8]) -> usize {
        let copy_len = core::cmp::min(N, data.len());
        
        #[cfg(not(feature = "simulator"))]
        unsafe {
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.buffer.as_mut_ptr() as *mut u8,
                copy_len
            );
        }
        
        #[cfg(feature = "simulator")]
        self.buffer[..copy_len].copy_from_slice(&data[..copy_len]);
        
        self.len = copy_len;
        copy_len
    }
} 