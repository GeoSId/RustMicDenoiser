package com.geosid.rustmicdenoiser.data.repository

import com.geosid.rustmicdenoiser.data.datasource.AudioConfig
import com.geosid.rustmicdenoiser.data.datasource.NativeRecorder

class RecordingRepository(
    private val nativeRecorder: NativeRecorder = NativeRecorder()
) {
    fun initialize() {
        nativeRecorder.initNative()
    }

    fun startRecording(activity: android.app.Activity): Result<String> {
        return nativeRecorder.startRecording(activity)
    }

    fun stopRecording(): Result<String> {
        return nativeRecorder.stopRecording()
    }

    fun setNoiseReductionEnabled(enabled: Boolean) {
        nativeRecorder.setNoiseReductionEnabled(enabled)
    }

    fun setAudioConfig(sampleRate: Int, bitDepth: Int, channels: Int, captureSampleRate: Int) {
        nativeRecorder.setAudioConfig(sampleRate, bitDepth, channels, captureSampleRate)
    }

    fun getAudioConfig(): AudioConfig? {
        return nativeRecorder.getAudioConfig()
    }

    fun cleanup() {
        nativeRecorder.cleanupNative()
    }
}
