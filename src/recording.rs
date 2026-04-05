use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use jni::objects::{JClass, JObject};
use jni::JNIEnv;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

use crate::denoise::AudioDenoiser;

// =============================================================================
// Audio Configuration Globals
// =============================================================================
// These static variables hold the audio configuration that can be set from the UI.
// They are thread-safe via Mutex and lazy-initialized.

/// Number of audio channels (1 = mono, 2 = stereo)
/// Stereo is default for better audio quality
static NUM_CHANNELS: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(2));

/// Bit depth for WAV output (8, 16, 24, 32)
/// 16-bit is standard for voice recording
static BITS_PER_SAMPLE: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(16));

/// Sample rate for the output WAV file
/// Common values: 8000 (telephone), 16000 (good for voice), 44100 (CD quality), 48000 (professional)
static RECORD_SAMPLE_RATE: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(16000));

/// Sample rate for hardware audio capture from microphone
/// Usually higher than output rate (44100 or 48000 on most Android devices)
static CAPTURE_SAMPLE_RATE: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(48000));

// =============================================================================
// Thread-Safe Stream Wrapper
// =============================================================================
// Wrapper to make cpal::Stream safe to store in a static variable.
// cpal::Stream is not Send/Sync by default, so we explicitly mark it as safe.

struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

// =============================================================================
// Recording State Management
// =============================================================================
// Global state for recording - uses Mutex for thread safety

/// Current recording state: either None (not recording) or Recording with file/samples
static RECORDING: Lazy<Mutex<RecordingState>> = Lazy::new(|| Mutex::new(RecordingState::None));

/// Active audio stream for recording - stored to keep it alive
static RECORDING_STREAM: Lazy<Mutex<Option<SendStream>>> = Lazy::new(|| Mutex::new(None));

/// Flag to enable/disable noise reduction during recording
static NOISE_REDUCTION_ENABLED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

// =============================================================================
// Recording State Enum
// =============================================================================
// Represents the current state of the audio recorder.

enum RecordingState {
    /// No recording in progress
    None,
    /// Currently recording audio
    Recording {
        file: std::fs::File,                 // File handle for writing
        path: String,                        // Path to the output WAV file
        samples: Vec<f32>,                   // Buffer to store captured audio samples
        denoiser: Arc<Mutex<AudioDenoiser>>, // Noise reduction processor
    },
}

// =============================================================================
// WAV File Header Writer
// =============================================================================
// Writes the WAV file header with the configured audio parameters.
/// Writes a standard WAV file header (RIFF header + fmt chunk + data chunk)
/// This header defines the audio format (sample rate, channels, bit depth, etc.)
/// The data_size parameter is the size of the audio data in bytes.
fn write_recording_wav_header<W: std::io::Write>(
    file: &mut W,
    data_size: u32,
) -> std::io::Result<()> {
    use std::io::Write;
    let sample_rate = *RECORD_SAMPLE_RATE.lock().unwrap();
    let num_channels = *NUM_CHANNELS.lock().unwrap();
    let bits_per_sample = *BITS_PER_SAMPLE.lock().unwrap();

    let file_size = data_size + 36;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;

    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    Ok(())
}

// =============================================================================
// Audio Sample Converter
// =============================================================================
// Converts normalized f32 samples to 16-bit PCM format for WAV file.

// Converts f32 audio samples (-1.0 to 1.0) to 16-bit signed integers
// and packs them into a byte vector for WAV file storage.
fn write_wav_samples(samples: &[f32]) -> Vec<u8> {
    let mut wav_data = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        // Clamp to valid range and convert to 16-bit integer
        let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        wav_data.extend_from_slice(&sample_i16.to_le_bytes());
    }
    wav_data
}

// =============================================================================
// Timestamp Generator
// =============================================================================
// Generates a simple Unix timestamp string for unique filenames.

// Generates a simple Unix timestamp (seconds since epoch)
// Used to create unique filenames for recordings.
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

// =============================================================================
// CPAL Audio Capture Setup
// =============================================================================
// Initializes and starts the audio input stream using cpal.
// This function sets up the microphone capture with the configured sample rate
// and channel count, then stores samples in the recording buffer.

// Sets up and starts the audio input stream using cpal (cross-platform audio library).
// The stream captures audio from the default input device (microphone) and stores
// samples in the RecordingState buffer. Audio is captured at the configured
// CAPTURE_SAMPLE_RATE and then downsampled to RECORD_SAMPLE_RATE.
fn start_standalone_recording_cpal() {
    let host = cpal::default_host();
    let device: cpal::Device = match host.default_input_device() {
        Some(d) => d,
        None => {
            log::error!("No input device available for recording");
            return;
        }
    };

    log::info!("Starting standalone recording via cpal");

    let capture_sample_rate = *CAPTURE_SAMPLE_RATE.lock().unwrap();
    let channels = *NUM_CHANNELS.lock().unwrap();

    let config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(capture_sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream: cpal::Stream = match device.build_input_stream(
        &config,
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            let mut rec_guard = match RECORDING.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            if let RecordingState::Recording { samples, .. } = &mut *rec_guard {
                let mut downsampled = Vec::with_capacity(data.len() / 3);
                let mut i = 0;
                while i + 2 < data.len() {
                    let sum = data[i] as i32 + data[i + 1] as i32 + data[i + 2] as i32;
                    downsampled.push((sum as f32 / 3.0) / 32768.0);
                    i += 3;
                }
                samples.extend(downsampled);
            }
        },
        move |err| {
            log::error!("Recording stream error: {}", err);
        },
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to build recording stream: {}", e);
            return;
        }
    };

    if let Err(e) = stream.play() {
        log::error!("Failed to start recording stream: {}", e);
        return;
    }

    let mut stream_guard = match RECORDING_STREAM.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    *stream_guard = Some(SendStream(stream));
    log::info!("Standalone recording stream started");
}

// =============================================================================
// JNI: Initialize Native Module
// =============================================================================
// Called when the native library is first loaded. Initializes Android logging.

// JNI export: Initializes the native module when library is loaded.
/// Initializes Android logger - called once when the native library is loaded.
/// This sets up logging so we can see output in logcat.
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_initNativeImpl(
    _env: JNIEnv,
    _class: JClass,
) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    log::info!("MainActivity_initNative: Recording module initialized");
}

// =============================================================================
// JNI: Start Recording
// =============================================================================
// Creates a new WAV file and starts capturing audio from the microphone.

// JNI export: Starts a new recording session.
/// Creates a new WAV file and begins capturing audio from the microphone.
/// Returns the path to the created recording file, or null on failure.
///
/// Workflow:
/// 1. Get the app's files directory from the Activity
/// 2. Create a new WAV file with appropriate header
/// 3. Initialize recording state with an AudioDenoiser
/// 4. Start the cpal audio input stream
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_startRecordingImpl<
    'a,
>(
    mut env: JNIEnv<'a>,
    _class: JClass<'a>,
    activity: JObject<'a>,
) -> jni::sys::jstring {
    let files_dir_obj = match env.call_method(&activity, "getFilesDir", "()Ljava/io/File;", &[]) {
        Ok(val) => match val.l() {
            Ok(obj) => obj,
            Err(e) => {
                log::error!("Failed to get files dir object: {:?}", e);
                return std::ptr::null_mut();
            }
        },
        Err(e) => {
            log::error!("Failed to call getFilesDir: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    let path_str_obj = match env.call_method(
        &files_dir_obj,
        "getAbsolutePath",
        "()Ljava/lang/String;",
        &[],
    ) {
        Ok(val) => match val.l() {
            Ok(obj) => obj,
            Err(e) => {
                log::error!("Failed to get path object: {:?}", e);
                return std::ptr::null_mut();
            }
        },
        Err(e) => {
            log::error!("Failed to call getAbsolutePath: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    let path_string: String = match env.get_string(&path_str_obj.into()) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Failed to get path string: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    let base_path = std::path::PathBuf::from(path_string);
    let timestamp = chrono_lite_timestamp();
    let filename = format!("recording_{}.wav", timestamp);
    let file_path = base_path.join(&filename);

    match std::fs::File::create(&file_path) {
        Ok(mut file) => {
            if let Err(e) = write_recording_wav_header(&mut file, u32::MAX) {
                log::error!("Failed to write WAV header: {}", e);
                return std::ptr::null_mut();
            }

            let mut rec_guard = RECORDING.lock().unwrap();
            *rec_guard = RecordingState::Recording {
                file,
                path: file_path.to_string_lossy().to_string(),
                samples: Vec::new(),
                denoiser: Arc::new(Mutex::new(AudioDenoiser::new())),
            };
            log::info!(
                "Recording started with noise reduction: {}",
                file_path.display()
            );

            drop(rec_guard);
            start_standalone_recording_cpal();

            match env.new_string(&file_path.to_string_lossy().to_string()) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(e) => {
            log::error!("Failed to create recording file: {}", e);
            std::ptr::null_mut()
        }
    }
}

// =============================================================================
// JNI: Stop Recording
// =============================================================================
// Stops the audio capture, applies optional noise reduction, and saves the WAV file.

// JNI export: Stops the current recording and saves the audio to disk.
/// Stops the audio stream, applies noise reduction if enabled, and writes
/// the final WAV file with proper header.
/// Returns the path to the saved recording file.
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_stopRecordingImpl<
    'a,
>(
    env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> jni::sys::jstring {
    let (path, samples, denoiser_arc) = {
        let rec_guard = RECORDING.lock().unwrap();
        match &*rec_guard {
            RecordingState::Recording {
                path,
                samples,
                denoiser,
                ..
            } => (path.clone(), samples.clone(), denoiser.clone()),
            RecordingState::None => {
                return std::ptr::null_mut();
            }
        }
    };

    let final_samples = {
        let noise_reduction_enabled = *NOISE_REDUCTION_ENABLED.lock().unwrap();
        if noise_reduction_enabled {
            let mut denoiser = denoiser_arc.lock().unwrap();
            let denoised = denoiser.process(&samples);
            log::info!(
                "Noise reduction applied: {} -> {} samples",
                samples.len(),
                denoised.len()
            );
            denoised
        } else {
            log::info!(
                "Noise reduction disabled, saving raw audio: {} samples",
                samples.len()
            );
            samples
        }
    };

    let wav_data = write_wav_samples(&final_samples);
    let data_size = wav_data.len() as u32;

    let mut file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to open file for writing: {}", e);
            return std::ptr::null_mut();
        }
    };

    use std::io::{BufWriter, Write};
    let mut writer = BufWriter::new(&mut file);

    if let Err(e) = write_recording_wav_header(&mut writer, data_size) {
        log::error!("Failed to write WAV header: {}", e);
        return std::ptr::null_mut();
    }

    if let Err(e) = writer.write_all(&wav_data) {
        log::error!("Failed to write audio data: {}", e);
        return std::ptr::null_mut();
    }

    if let Err(e) = writer.flush() {
        log::error!("Failed to flush: {}", e);
        return std::ptr::null_mut();
    }

    if let Err(e) = writer.into_inner().unwrap().sync_all() {
        log::error!("Failed to sync: {}", e);
        return std::ptr::null_mut();
    }

    log::info!(
        "Recording saved: {} ({} samples)",
        path,
        final_samples.len()
    );

    {
        let mut rec_guard = RECORDING.lock().unwrap();
        *rec_guard = RecordingState::None;
    }

    let mut stream_guard = match RECORDING_STREAM.lock() {
        Ok(guard) => guard,
        Err(_) => return std::ptr::null_mut(),
    };
    *stream_guard = None;
    log::info!("Recording stream stopped");

    match env.new_string(&path) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// =============================================================================
// JNI: Set Noise Reduction
// =============================================================================
// Enables or disables noise reduction when saving the recording.

// JNI export: Sets whether to apply noise reduction when stopping recording.
/// Controls whether noise reduction is applied when stopRecording() is called.
/// If true, the nnnoiseless denoiser will process the audio before saving.
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_setNoiseReductionEnabledImpl(
    _env: JNIEnv,
    _class: JClass,
    enabled: bool,
) {
    let mut guard = NOISE_REDUCTION_ENABLED.lock().unwrap();
    *guard = enabled;
    log::info!("Noise reduction set to: {}", enabled);
}

// =============================================================================
// JNI: Cleanup
// =============================================================================
// Releases resources and resets the recording state.

// JNI export: Cleanup when the recorder is no longer needed.
/// Releases the audio stream and resets all recording state.
/// Call this when the recorder is destroyed or when switching audio settings.
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_cleanupNative(
    _env: JNIEnv,
    _class: JClass,
) {
    log::info!("MainActivity_cleanupNative: Cleaning up");
    let mut rec_guard = RECORDING.lock().unwrap();
    *rec_guard = RecordingState::None;
    drop(rec_guard);
    let mut stream_guard = RECORDING_STREAM.lock().unwrap();
    *stream_guard = None;
}

// =============================================================================
// JNI: Set Audio Configuration
// =============================================================================
// Configures audio parameters for recording - can be called from UI.

// JNI export: Sets audio configuration parameters from the UI.
/// Configures the audio settings for recording. Call this before startRecording().
///
/// Parameters:
/// - sample_rate: Output WAV file sample rate (8000, 16000, 44100, 48000)
/// - bit_depth: Bits per sample (8, 16, 24, 32)
/// - channels: Number of channels (1 = mono, 2 = stereo)
/// - capture_sample_rate: Hardware sample rate (44100 or 48000 on most Android)
///
/// Example: setAudioConfig(16000, 16, 1, 48000)
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_setAudioConfigImpl(
    _env: JNIEnv,
    _class: JClass,
    sample_rate: u32,
    bit_depth: u32,
    channels: u32,
    capture_sample_rate: u32,
) {
    {
        let mut guard = RECORD_SAMPLE_RATE.lock().unwrap();
        *guard = sample_rate;
    }
    {
        let mut guard = BITS_PER_SAMPLE.lock().unwrap();
        *guard = bit_depth as u16;
    }
    {
        let mut guard = NUM_CHANNELS.lock().unwrap();
        *guard = channels as u16;
    }
    {
        let mut guard = CAPTURE_SAMPLE_RATE.lock().unwrap();
        *guard = capture_sample_rate;
    }
    log::info!(
        "Audio config set: sample_rate={}, bit_depth={}, channels={}, capture_rate={}",
        sample_rate,
        bit_depth,
        channels,
        capture_sample_rate
    );
}

// =============================================================================
// JNI: Get Audio Configuration
// =============================================================================
// Returns the current audio configuration settings.

// JNI export: Gets the current audio configuration.
/// Returns the current audio settings as a comma-separated string:
/// "sampleRate,bitDepth,channels,captureSampleRate"
/// Useful for displaying current settings in the UI.
#[no_mangle]
pub unsafe extern "system" fn Java_com_geosid_rustmicdenoiser_data_datasource_NativeRecorder_getAudioConfigImpl(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    let sample_rate = *RECORD_SAMPLE_RATE.lock().unwrap();
    let bit_depth = *BITS_PER_SAMPLE.lock().unwrap() as u32;
    let channels = *NUM_CHANNELS.lock().unwrap() as u32;
    let capture_sample_rate = *CAPTURE_SAMPLE_RATE.lock().unwrap();

    let config_str = format!(
        "{},{},{},{}",
        sample_rate, bit_depth, channels, capture_sample_rate
    );

    match env.new_string(&config_str) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
