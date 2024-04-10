if you need libvulkan to debug properly,

cp ~/VulkanSDK/1.3.275.0/macOS/lib/libvulkan.1.3.275.dylib target/debug/
cd target/debug
ln -s libvulkan.1.3.275.dylib libvulkan.1.dylib

nice run command

RUST_BACKTRACE=1 ALWAYS_ON_TOP=true cargo-watch -x run
