# RustMicDenoiser

A microphone noise reduction app for Android, built with Rust for high-performance audio processing. The denoiser targets **background white noise** (steady hiss, fan hum, and similar broadband noise) so speech comes through clearer.

## Project Overview

This project demonstrates how to build an audio denoising application for Android using:
- **Rust** for native audio processing
- **cpal** for cross-platform audio handling
- **nnnoiseless** for RNNoise-based noise suppression
- **JNI** to communicate between Rust native code and Android Java/Kotlin

## Audio Processing Flow

```
Microphone Input → cpals Audio Capture → nnnoiseless Denoising → Output Audio
```

1. **Audio Capture**: The app captures audio from the device microphone using `cpal`
2. **Noise Suppression**: The audio stream is processed through the `nnnoiseless` noise suppressor (RNNoise-based)
3. **Output**: The cleaned audio is sent to the device speaker (or can be streamed elsewhere)

This pipeline processes audio with minimal latency while effectively reducing background noise, including **white noise** from the environment.

## Audio Samples

This project includes three sample audio files demonstrating the denoising effect:

| File | Description |
|------|-------------|
| [white_noice.wav](white_noice.wav) | Pure white noise (background hiss) |
| [with_white_noice.wav](with_white_noice.wav) | Audio with white noise overlaid |
| [no_white_noice.wav](no_white_noice.wav) | Clean audio after denoising |


## Why Rust?

- **Performance**: Rust provides near-native performance with zero-cost abstractions, crucial for audio processing
- **Memory Safety**: Rust's ownership system eliminates buffer overflow bugs that could crash the app during audio processing
- **No Garbage Collection**: Unlike Java/Kotlin, Rust doesn't have garbage collection pauses that could cause audio glitches
- **Small Binary Size**: Produces minimal binaries ideal for mobile apps
- **Thread Safety**: Safe concurrency primitives help with parallel audio processing

## Key Libraries

### cpal
Cross-platform audio library for Rust. It provides a simple API to:
- List available audio input/output devices
- Capture audio from microphones
- Play audio to speakers
- Configure sample rates, channels, and formats

```rust
let host = cpal::default_host();
let device = host.default_input_device().unwrap();
```

### nnnoiseless
A Rust implementation of the RNNoise noise suppression library. It:
- Uses deep learning to suppress background noise, including white noise (constant broadband hiss)
- Requires minimal CPU resources
- Works on PCM audio samples
- Provides excellent noise reduction quality

## Build Configuration

### 1. build.sh

This script builds the Rust library for Android and packages it into an APK. It performs the following steps:

1. Sets up Android SDK and NDK environment variables
2. Builds the Rust library (`libdenoise`) for Android ARM64 using cargo ndk
3. Copies the compiled `.so` file to the jniLibs directory
4. Builds the Android APK using Gradle
5. Installs the APK to a connected Android device (optional)

**Important**: You need to edit `build.sh` and replace the paths with your own project paths:

```bash
export ANDROID_HOME=/path/to/your/Android/Sdk
export PATH=$PATH:/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin
export ORT_DYLIB_PATH=/path/to/your/project/app/src/main/jniLibs/arm64-v8a
# Update the cd and cp commands with your project path
cd /path/to/your/project
```

### 2. .cargo/config.toml

This Cargo configuration file sets up the Android NDK toolchain and environment variables.

**Important**: You need to edit `.cargo/config.toml` and replace all `/path/to/your` paths with your own:
- Replace `/path/to/your/Android/Sdk` with your actual Android SDK path
- Replace `/path/to/your/project` with your actual project path

```toml
[target.aarch64-linux-android]
linker = "/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"

[env]
CC_aarch64_linux_android = "/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
CXX_aarch64_linux_android = "/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang++"
AR_aarch64_linux_android = "/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
ORT_LIB_LOCATION = "/path/to/your/project/app/src/main/jniLibs/arm64-v8a"
ORT_INCLUDE_DIR = "/path/to/your/project/app/src/main/jniLibs"
ANDROID_NDK_HOME = "/path/to/your/Android/Sdk/ndk/26.1.10909125"
ANDROID_NDK = "/path/to/your/Android/Sdk/ndk/26.1.10909125"
BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android = "--sysroot=/path/to/your/Android/Sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/sysroot"

[build]
target = "aarch64-linux-android"
```

## Prerequisites

1. **Android SDK**: Installed and configured
2. **NDK 26.1.10909125**: Required for cross-compiling Rust to Android
3. **Rust**: Install via rustup with Android target:
   ```bash
   rustup target add aarch64-linux-android
   ```
4. **nnnoiseless model files**: Located in `app/src/main/jniLibs/arm64-v8a`

## Building

```bash
./build.sh
```

This will:
1. Build the Rust library for Android ARM64
2. Copy the `.so` file to jniLibs
3. Build the Android APK
4. Install to connected device

## Dependencies in Cargo.toml

- **cpal**: Cross-platform audio input/output
- **nnnoiseless**: RNNoise-based noise suppression
- **jni**: Java Native Interface for Android communication
- **android_logger**: Logging for Android
- **once_cell**: Lazy static initialization
- **log**: Logging facade
