package com.geosid.rustmicdenoiser.data.datasource

import android.media.MediaPlayer
import java.io.File

class AudioPlayer {
    private var mediaPlayer: MediaPlayer? = null
    private var onCompletionListener: (() -> Unit)? = null
    private var onProgressListener: ((Long, Long) -> Unit)? = null
    private var isProgressUpdating = false

    fun play(filePath: String, onComplete: () -> Unit = {}) {
        stop()
        onCompletionListener = onComplete
        
        val file = File(filePath)
        if (!file.exists()) {
            return
        }

        mediaPlayer = MediaPlayer().apply {
            setDataSource(filePath)
            setOnPreparedListener {
                start()
                startProgressUpdates()
            }
            setOnCompletionListener {
                stopProgressUpdates()
                onCompletionListener?.invoke()
            }
            setOnErrorListener { _, _, _ ->
                stopProgressUpdates()
                true
            }
            prepareAsync()
        }
    }

    fun pause() {
        mediaPlayer?.let {
            if (it.isPlaying) {
                it.pause()
                stopProgressUpdates()
            }
        }
    }

    fun resume() {
        mediaPlayer?.let {
            if (!it.isPlaying) {
                it.start()
                startProgressUpdates()
            }
        }
    }

    fun stop() {
        stopProgressUpdates()
        mediaPlayer?.apply {
            try {
                if (isPlaying) {
                    stop()
                }
                release()
            } catch (e: Exception) {
                // Ignore
            }
        }
        mediaPlayer = null
    }

    fun seekTo(positionMs: Long) {
        mediaPlayer?.seekTo(positionMs.toInt())
    }

    fun isPlaying(): Boolean = mediaPlayer?.isPlaying == true

    fun getCurrentPosition(): Long = mediaPlayer?.currentPosition?.toLong() ?: 0L

    fun getDuration(): Long = mediaPlayer?.duration?.toLong() ?: 0L

    fun setOnProgressListener(listener: (Long, Long) -> Unit) {
        onProgressListener = listener
    }

    private fun startProgressUpdates() {
        if (isProgressUpdating) return
        isProgressUpdating = true
        Thread {
            while (isProgressUpdating && mediaPlayer?.isPlaying == true) {
                val pos = getCurrentPosition()
                val dur = getDuration()
                onProgressListener?.invoke(pos, dur)
                Thread.sleep(100)
            }
        }.start()
    }

    private fun stopProgressUpdates() {
        isProgressUpdating = false
    }

    fun release() {
        stop()
        onProgressListener = null
        onCompletionListener = null
    }
}
