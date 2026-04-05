#!/bin/bash

set -e

export ANDROID_HOME=/path/to/your/Android/Sdk
export PATH=$PATH:/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin
export ORT_DYLIB_PATH=/path/to/your/project/RustMicDenoiser/app/src/main/jniLibs/arm64-v8a
export ORT_SKIP_DOWNLOAD=1

echo "=== Building Rust native library ==="
cd /path/to/your/project
cargo build --target aarch64-linux-android --release

echo "=== Copying .so files to jniLibs ==="
cp target/aarch64-linux-android/release/libRustMicDenoiser.so app/src/main/jniLibs/arm64-v8a/

echo "=== Building Android APK ==="
./gradlew assembleDebug

echo "=== Installing to device ==="
adb install -r app/build/outputs/apk/debug/app-debug.apk

echo "=== Build complete! ==="
