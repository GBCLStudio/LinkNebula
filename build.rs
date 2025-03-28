fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    
    // 配置编译环境和目标平台特定设置
    #[cfg(feature = "bearpi")]
    {
        println!("cargo:rustc-link-arg=-Tbearpi_hi2821.ld");
        println!("cargo:rustc-link-arg=-nostartfiles");
    }
} 