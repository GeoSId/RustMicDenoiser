use nnnoiseless::DenoiseState;

const FRAME_SIZE: usize = 480; // 30ms at 16kHz //30ms of audio (480/16000 = 0.03s)

pub struct AudioDenoiser {
    denoiser: Box<DenoiseState<'static>>,
    buffer: Vec<f32>, // Remainder buffer //remainder	Stores incomplete frames for next call
}

impl AudioDenoiser {
    pub fn new() -> Self {
        Self {
            denoiser: DenoiseState::new(),
            buffer: Vec::with_capacity(FRAME_SIZE),
        }
    }

    pub fn process(&mut self, samples_16k: &[f32]) -> Vec<f32> {
        // 1. Process any remainder from previous call (incomplete frame)
        if samples_16k.is_empty() {
            // Fill buffer to FRAME_SIZE, process frame, clear buffer
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current_idx = 0;

        // Process any remainder from previous calls
        if !self.buffer.is_empty() {
            // Has leftover from last call?
            let needed = FRAME_SIZE - self.buffer.len(); // How many more to fill frame
            if samples_16k.len() >= needed {
                // Enough new samples to complete frame?
                self.buffer.extend_from_slice(&samples_16k[..needed]); // Fill buffer
                current_idx += needed;

                // Create and process frame
                let mut frame = [0.0f32; FRAME_SIZE];
                frame.copy_from_slice(&self.buffer);
                self.buffer.clear();

                for sample in &mut frame {
                    // Scale f32 → int range
                    *sample *= 32767.0;
                }

                let mut denoised = [0.0f32; FRAME_SIZE];
                self.denoiser.process_frame(&mut denoised, &frame);

                for sample in &mut denoised {
                    // Scale back f32
                    *sample /= 32767.0;
                }

                result.extend_from_slice(&denoised);
            } else {
                // Not enough samples, add all and wait for next call
                self.buffer.extend_from_slice(samples_16k);
                return result;
            }

            //EXAMPLE
            //Last call: sent 100 samples (buffer = 100)
            //This call: send 400 samples (need 380 more to reach 480)
            //Step 1: buffer = [100] + [380] = 480 ✓
            //Step 2: Process full frame
            //Step 3: current_idx = 380 (skip first 380 of new 400)
            //Step 4: buffer = 0 (cleared)

            //Early return:
            //If not enough new samples to complete frame
            //Store all new samples in buffer
            //Return empty result, wait for next call
        }

        // Process full frames directly
        let remaining = samples_16k.len() - current_idx; // Samples left after processing buffer
        let frames = remaining / FRAME_SIZE; // How many full frames fit

        //Example:
        //Input: 2000 samples
        //Buffer processed: 380 samples (used to complete previous frame)
        //remaining = 2000 - 380 = 1620
        //frames = 1620 / 480 = 3 full frames (1440 samples)

        //What happens next:
        result.reserve(frames * FRAME_SIZE); // Pre-allocate output
                                             // Process each frame
        for _ in 0..frames {
            // 1. Extract 480 samples
            let mut frame = [0.0f32; FRAME_SIZE]; // Extract 480 samples
            frame.copy_from_slice(&samples_16k[current_idx..current_idx + FRAME_SIZE]);
            current_idx += FRAME_SIZE;
            // 2. Scale to integer range (required by denoiser)
            for sample in &mut frame {
                *sample *= 32767.0; // f32 (-1 to 1) → f32 (large range)
            }
            // 3. Denoise
            let mut denoised = [0.0f32; FRAME_SIZE];
            self.denoiser.process_frame(&mut denoised, &frame);
            // 4. Scale back to normalized range
            for sample in &mut denoised {
                *sample /= 32767.0; // f32 → f32 (-1 to 1)
            }

            result.extend_from_slice(&denoised);
        }

        // Store new remainder
        // 6. Store remainder (< 480 samples) for next call
        if current_idx < samples_16k.len() {
            self.buffer.extend_from_slice(&samples_16k[current_idx..]);
        }

        // Input samples:  [0.....|....480....|....960....|....1440...|...1620]
        //                        frame 1        frame 2        frame 3   remainder
        //Processing:    extract → scale → denoise → scale → add to result
        //Result:        [480 denoised] [480 denoised] [480 denoised] + buffer = 180

        result
    }
}

impl Default for AudioDenoiser {
    fn default() -> Self {
        Self::new()
    }
}
