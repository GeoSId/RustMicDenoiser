package com.geosid.rustmicdenoiser

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import androidx.lifecycle.ViewModelProvider
import com.geosid.rustmicdenoiser.ui.screen.RecordingScreen
import com.geosid.rustmicdenoiser.ui.screen.RecordingViewModel
import com.geosid.rustmicdenoiser.ui.theme.RustMicDenoiserTheme

class MainActivity : ComponentActivity() {

    private val viewModel: RecordingViewModel by viewModels(
        factoryProducer = {
            ViewModelProvider.AndroidViewModelFactory.getInstance(application)
        }
    )

    private val requestPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { isGranted: Boolean ->
        if (!isGranted) {
            viewModel.setPermissionDenied(true)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        
        checkAndRequestPermission()
        
        setContent {
            RustMicDenoiserTheme {
                RecordingApp(viewModel = viewModel)
            }
        }
    }

    private fun checkAndRequestPermission() {
        when {
            ContextCompat.checkSelfPermission(
                this,
                Manifest.permission.RECORD_AUDIO
            ) == PackageManager.PERMISSION_GRANTED -> {
                // Permission already granted
            }
            else -> {
                requestPermissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
            }
        }
    }
}

@Composable
fun RecordingApp(
    viewModel: RecordingViewModel
) {
    Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
        RecordingScreen(
            modifier = Modifier.padding(innerPadding),
            viewModel = viewModel
        )
    }
}
