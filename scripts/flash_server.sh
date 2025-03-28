#!/bin/bash
set -e

echo "正在烧录AetherLink服务端固件..."

# 检查设备连接
echo "检查设备连接..."
if ! lsusb | grep -q "BearPi"; then
    echo "错误：未检测到BearPi设备，请检查连接"
    exit 1
fi

# 烧录服务端固件
echo "烧录服务端固件..."
openocd -f interface/cmsis-dap.cfg -f target/hi2821.cfg -c "program ../target/thumbv7em-none-eabihf/release/server verify reset exit"

echo "服务端固件烧录完成！" 