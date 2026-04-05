package com.geosid.rustmicdenoiser.ui.screen

import android.app.Activity
import android.media.MediaPlayer
import androidx.lifecycle.AndroidViewModel
import com.geosid.rustmicdenoiser.data.datasource.AudioConfig
import com.geosid.rustmicdenoiser.data.datasource.AudioPlayer
import com.geosid.rustmicdenoiser.data.repository.RecordingRepository
import com.geosid.rustmicdenoiser.domain.model.Recording
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import java.io.File

enum class PlaybackState {
    IDLE, PLAYING, PAUSED
}

data class RecordingUiState(
    val isRecording: Boolean = false,
    val isPaused: Boolean = false,
    val recordingDurationMs: Long = 0L,
    val recordings: List<Recording> = emptyList(),
    val currentPlayingId: String? = null,
    val playbackState: PlaybackState = PlaybackState.IDLE,
    val playbackPositionMs: Long = 0L,
    val playbackDurationMs: Long = 0L,
    val errorMessage: String? = null,
    val permissionDenied: Boolean = false,
    val noiseReductionEnabled: Boolean = true,
    val audioConfig: AudioConfig = AudioConfig(16000, 16, 1, 48000),
    val showSettings: Boolean = false
)

class RecordingViewModel(
    application: android.app.Application,
    private val repository: RecordingRepository = RecordingRepository()
) : AndroidViewModel(application) {

    private val _uiState = MutableStateFlow(RecordingUiState())
    val uiState: StateFlow<RecordingUiState> = _uiState.asStateFlow()

    private val audioPlayer = AudioPlayer()
    private var recordingStartTime: Long = 0L

    init {
        repository.initialize()
        loadRecordings()
        setupAudioPlayer()
        loadAudioConfig()
    }

    private fun loadAudioConfig() {
        val config = repository.getAudioConfig()
        _uiState.update { it.copy(audioConfig = config ?: AudioConfig(16000, 16, 1, 48000)) }
    }

    private fun loadRecordings() {
        val filesDir = getApplication<android.app.Application>().filesDir
        
        val files = filesDir.listFiles { file -> file.extension == "wav" }
            ?.sortedByDescending { it.lastModified() }
            ?: emptyList()
        
        val recordings = files.map { file ->
            Recording(
                id = file.nameWithoutExtension,
                filePath = file.absolutePath,
                fileName = file.name,
                durationMs = getAudioDuration(file.absolutePath),
                createdAt = file.lastModified()
            )
        }
        
        _uiState.update { it.copy(recordings = recordings) }
    }

    private fun getAudioDuration(filePath: String): Long {
        return try {
            val player = MediaPlayer()
            player.setDataSource(filePath)
            player.prepare()
            val duration = player.duration.toLong()
            player.release()
            duration
        } catch (e: Exception) {
            0L
        }
    }

    private fun setupAudioPlayer() {
        audioPlayer.setOnProgressListener { position, duration ->
            _uiState.update {
                it.copy(
                    playbackPositionMs = position,
                    playbackDurationMs = duration
                )
            }
        }
    }

    fun startRecording(activity: Activity) {
        repository.startRecording(activity)
            .onSuccess { _ ->
                recordingStartTime = System.currentTimeMillis()
                _uiState.update {
                    it.copy(
                        isRecording = true,
                        isPaused = false,
                        recordingDurationMs = 0L,
                        errorMessage = null
                    )
                }
            }
            .onFailure { error ->
                _uiState.update {
                    it.copy(errorMessage = error.message)
                }
            }
    }

    fun pauseRecording() {
        _uiState.update { it.copy(isPaused = true) }
    }

    fun resumeRecording() {
        _uiState.update { it.copy(isPaused = false) }
    }

    fun stopRecording() {
        repository.stopRecording()
            .onSuccess { _ ->
                _uiState.update {
                    it.copy(
                        isRecording = false,
                        isPaused = false,
                        errorMessage = null
                    )
                }
                loadRecordings()
            }
            .onFailure { error ->
                _uiState.update {
                    it.copy(errorMessage = error.message)
                }
            }
    }

    fun playRecording(recording: Recording) {
        stopPlayback()
        
        _uiState.update {
            it.copy(
                currentPlayingId = recording.id,
                playbackState = PlaybackState.PLAYING,
                playbackPositionMs = 0L,
                playbackDurationMs = recording.durationMs
            )
        }
        
        audioPlayer.play(recording.filePath) {
            _uiState.update {
                it.copy(
                    playbackState = PlaybackState.IDLE,
                    currentPlayingId = null,
                    playbackPositionMs = 0L
                )
            }
        }
    }

    fun pausePlayback() {
        audioPlayer.pause()
        _uiState.update { it.copy(playbackState = PlaybackState.PAUSED) }
    }

    fun resumePlayback() {
        audioPlayer.resume()
        _uiState.update { it.copy(playbackState = PlaybackState.PLAYING) }
    }

    fun stopPlayback() {
        audioPlayer.stop()
        _uiState.update {
            it.copy(
                playbackState = PlaybackState.IDLE,
                currentPlayingId = null,
                playbackPositionMs = 0L,
                playbackDurationMs = 0L
            )
        }
    }

    fun seekPlayback(positionMs: Long) {
        audioPlayer.seekTo(positionMs)
        _uiState.update { it.copy(playbackPositionMs = positionMs) }
    }

    fun deleteRecording(recording: Recording) {
        if (_uiState.value.currentPlayingId == recording.id) {
            stopPlayback()
        }
        
        try {
            File(recording.filePath).delete()
        } catch (e: Exception) {
            // Ignore
        }
        
        loadRecordings()
    }

    fun setNoiseReductionEnabled(enabled: Boolean) {
        repository.setNoiseReductionEnabled(enabled)
        _uiState.update { it.copy(noiseReductionEnabled = enabled) }
    }

    fun toggleNoiseReduction() {
        val newValue = !_uiState.value.noiseReductionEnabled
        setNoiseReductionEnabled(newValue)
    }

    fun clearError() {
        _uiState.update { it.copy(errorMessage = null) }
    }

    fun setPermissionDenied(denied: Boolean) {
        _uiState.update { it.copy(permissionDenied = denied) }
    }

    fun updateDuration(durationMs: Long) {
        _uiState.update { it.copy(recordingDurationMs = durationMs) }
    }

    fun toggleSettings() {
        _uiState.update { it.copy(showSettings = !it.showSettings) }
    }

    fun updateAudioConfig(config: AudioConfig) {
        repository.setAudioConfig(config.sampleRate, config.bitDepth, config.channels, config.captureSampleRate)
        _uiState.update { it.copy(audioConfig = config) }
    }

    override fun onCleared() {
        super.onCleared()
        audioPlayer.release()
        repository.cleanup()
    }
}
