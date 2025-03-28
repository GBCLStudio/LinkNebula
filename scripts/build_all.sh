#!/bin/bash
set -e

echo "正在编译AetherLink所有固件..."

# 编译客户端固件
echo "编译客户端..."
cargo build --release --package client

# 编译转发节点固件
echo "编译转发节点..."
cargo build --release --package forward

# 编译服务端固件
echo "编译服务端..."
cargo build --release --package server

echo "所有固件编译完成！" 