use super::math::*;

pub trait Process<T, U> {
    fn process(&self, sig: &[T], output: &mut[U], scale: f32);
}

pub trait ProcessMut<T, U> {
    fn process(&mut self, sig: &[T], output: &mut[U], scale: f32);
}

pub struct PageLog {
    led_bin_map: Vec<usize>
}

impl PageLog {
    // Shifted rounding scheme to reduce smearing
    pub fn new(fft_size: u32, sample_rate: f32, max_frequency: f32, num_leds: usize) -> Self {
        let base = freq_to_bin(max_frequency, fft_size as f32, sample_rate) as f32;

        let bins = (0..num_leds).map(|idx| {
            let b1 = f32::min(base.powf(idx as f32 / ((num_leds as f32) - 1.0)), (fft_size as f32/ 2.0) - 1.0);
            let b1 = (b1 + (idx as f32)).round() as usize;
            b1
        }).collect();

        PageLog {
            led_bin_map: bins
        }
    }
}

impl Process<f32, f32> for PageLog {
    fn process(&self, sig: &[f32], output: &mut[f32], scale: f32) {
        let mut b0 = 0;
        for (&bin, o) in self.led_bin_map.iter().zip(output.iter_mut()) {
            let diff = usize::max(bin-b0, 1);
            let val = sig.iter().skip(b0).take(diff).fold(0.0, |acc, &x| return acc + x) / diff as f32;

            //let val_norm = (f32::min(val / scale, 1.0) * 255.0) as u8;
            let val_norm = f32::min(val * scale, 1.0);
            *o = val_norm;
            b0 = bin;
        }
    }
}


pub struct ExpDecay {
    memory: Vec<f32>,
    decay_rate: f32,
    max: f32
}

impl ExpDecay {
    pub fn new(cap: usize, sample_rate: f32, decay_time: f32, max: f32) -> Self {
        ExpDecay {
            memory: vec![0.0; cap],
            decay_rate: (1.0 / (- decay_time * sample_rate)).exp(),
            max
        }
    }
}

impl ProcessMut<f32, f32> for ExpDecay {
    fn process(&mut self, sig: &[f32], output: &mut[f32], _: f32) {
        for (&i, (o, m)) in sig.iter().zip(output.iter_mut().zip(self.memory.iter_mut())) {
            let mut tmp = self.decay_rate * *m;
            tmp += i;
            if tmp < 0.005 {tmp = 0.0}
            tmp = f32::min(tmp, self.max);

            *m = tmp;
            *o = tmp;
        }
    }
}