package com.geosid.rustmicdenoiser.domain.model

data class Recording(
    val id: String,
    val filePath: String,
    val fileName: String,
    val durationMs: Long,
    val createdAt: Long = System.currentTimeMillis()
)
