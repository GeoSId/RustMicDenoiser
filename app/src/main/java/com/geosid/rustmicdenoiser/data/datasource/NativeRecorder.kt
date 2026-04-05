package com.geosid.rustmicdenoiser.data.datasource

class NativeRecorder {
    companion object {

        fun loadLibrary() {
            System.loadLibrary("RustMicDenoiser")
        }
    }

    private external fun initNativeImpl()
    private external fun startRecordingImpl(activity: android.app.Activity): String?
    private external fun stopRecordingImpl(): String?
    private external fun setNoiseReductionEnabledImpl(enabled: Boolean)
    private external fun cleanupNativeImpl()
    private external fun setAudioConfigImpl(sampleRate: Int, bitDepth: Int, channels: Int, captureSampleRate: Int): Boolean
    private external fun getAudioConfigImpl(): String?

    fun initNative() {
        loadLibrary()
        initNativeImpl()
    }

    fun startRecording(activity: android.app.Activity): Result<String> {
        return try {
            val path = startRecordingImpl(activity)
            if (path != null) {
                Result.success(path)
            } else {
                Result.failure(Exception("Failed to start recording"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun stopRecording(): Result<String> {
        return try {
            val path = stopRecordingImpl()
            if (path != null) {
                Result.success(path)
            } else {
                Result.failure(Exception("Failed to stop recording"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun setNoiseReductionEnabled(enabled: Boolean) {
        setNoiseReductionEnabledImpl(enabled)
    }

    fun cleanupNative() {
        cleanupNativeImpl()
    }

    fun setAudioConfig(sampleRate: Int, bitDepth: Int, channels: Int, captureSampleRate: Int): Boolean {
        return setAudioConfigImpl(sampleRate, bitDepth, channels, captureSampleRate)
    }

    fun getAudioConfig(): AudioConfig? {
        return try {
            val configStr = getAudioConfigImpl()
            if (configStr != null) {
                val parts = configStr.split(",")
                if (parts.size == 4) {
                    AudioConfig(
                        sampleRate = parts[0].toInt(),
                        bitDepth = parts[1].toInt(),
                        channels = parts[2].toInt(),
                        captureSampleRate = parts[3].toInt()
                    )
                } else null
            } else null
        } catch (e: Exception) {
            null
        }
    }
}

data class AudioConfig(
    val sampleRate: Int,
    val bitDepth: Int,
    val channels: Int,
    val captureSampleRate: Int
)
