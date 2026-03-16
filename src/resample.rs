/// Stateful linear-interpolation resampler.
/// Carries fractional position and last sample across calls to avoid boundary discontinuities.
pub struct Resampler {
    from_rate: u32,
    to_rate: u32,
    frac_pos: f64,
    last_sample: i16,
}

impl Resampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        Self {
            from_rate,
            to_rate,
            frac_pos: 0.0,
            last_sample: 0,
        }
    }

    pub fn needs_resample(&self) -> bool {
        self.from_rate != self.to_rate
    }

    pub fn process(&mut self, input: &[i16]) -> Vec<i16> {
        if !self.needs_resample() {
            return input.to_vec();
        }

        let ratio = self.from_rate as f64 / self.to_rate as f64;
        let estimated_len = (input.len() as f64 / ratio) as usize + 2;
        let mut output = Vec::with_capacity(estimated_len);

        while (self.frac_pos as usize) < input.len() {
            let idx = self.frac_pos as usize;
            let frac = self.frac_pos - idx as f64;

            let s0 = if idx == 0 {
                self.last_sample
            } else {
                input[idx - 1]
            };
            let s1 = input[idx];

            let interpolated = s0 as f64 * (1.0 - frac) + s1 as f64 * frac;
            output.push(interpolated.clamp(i16::MIN as f64, i16::MAX as f64) as i16);

            self.frac_pos += ratio;
        }

        // Carry over state for next chunk
        self.frac_pos -= input.len() as f64;
        if let Some(&last) = input.last() {
            self.last_sample = last;
        }

        output
    }
}
