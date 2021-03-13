use super::math::*;
use std::collections::VecDeque;

pub trait Process<T, U> {
    fn process(&self, sig: &[T], output: &mut[U], p: f32);
}

pub trait ProcessMut<T, U> {
    fn process(&mut self, sig: &[T], output: &mut[U], p: f32);
}

pub struct PageLog {
    led_bin_map: Vec<usize>
}

impl PageLog {
    // Shifted rounding scheme to reduce smearing
    pub fn new(fft_size: usize, sample_rate: f32, max_frequency: f32, num_leds: usize) -> Self {
        let base = f_to_bin(fft_size, sample_rate, max_frequency) as f32;

        let bins = (0..num_leds).map(|idx| {
            let b1 = f32::min(base.powf(idx as f32 / ((num_leds as f32) - 1.0)), (fft_size as f32/ 2.0) - 1.0);
            let b1 = (b1 + (idx as f32)).round() as usize;
            b1
        }).collect();

        PageLog {
            led_bin_map: bins
        }
    }

    pub fn get_bin(&self, led: usize) -> usize
    {
        self.led_bin_map[led]
    } 
}

impl Process<f32, f32> for PageLog {
    fn process(&self, sig: &[f32], output: &mut[f32], _: f32) {
        let mut b0 = 0;
        for (&bin, o) in self.led_bin_map.iter().zip(output.iter_mut()) {
            let diff = usize::max(bin-b0, 1);
            let val = sig.iter().skip(b0).take(diff).fold(0.0, |acc, &x| return acc + x) / diff as f32;

            //let val_norm = (f32::min(val / scale, 1.0) * 255.0) as u8;
            let val_norm = f32::min(val, 1.0);
            *o = val_norm;
            b0 = bin;
        }
    }
}

// pub struct PageLog {
//     led_bin_map: Vec<usize>
// }

// impl PageLog {
//     // Shifted rounding scheme to reduce smearing
//     pub fn new(fft_size: usize, sample_rate: f32, max_frequency: f32, num_leds: usize) -> Self {
//         let base = f_to_bin(fft_size, sample_rate, max_frequency) as f32;

//         let bins = (0..num_leds).rev().map(|idx| {

//             let root = 2f32.powf(idx as f32 / 12.0).recip();

//             //let b1 = f32::min(base.powf(idx as f32 / ((num_leds as f32) - 1.0)), (fft_size as f32/ 2.0) - 1.0);
//             let b1 = f32::min(base * root, (fft_size as f32/ 2.0) - 1.0);
//             let b1 = (b1 + (idx as f32)).round() as usize;
//             b1
//         }).collect();

//         println!("{:?}", bins);

//         PageLog {
//             led_bin_map: bins
//         }
//     }
// }

// impl Process<f32, f32> for PageLog {
//     fn process(&self, sig: &[f32], output: &mut[f32], _: f32) {
//         let mut b0 = 0;
//         for (&bin, o) in self.led_bin_map.iter().zip(output.iter_mut()) {
//             let diff = usize::max(bin-b0, 1);
//             let val = sig.iter().skip(b0).take(diff).fold(0.0, |acc, &x| return acc + x) / diff as f32;

//             //let val_norm = (f32::min(val / scale, 1.0) * 255.0) as u8;
//             let val_norm = f32::min(val, 1.0);
//             *o = val_norm;
//             b0 = bin;
//         }
//     }
// }

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
            if tmp < 0.0000005 {tmp = 0.0}
            tmp = f32::min(tmp, self.max);

            *m = tmp;
            *o = tmp;
        }
    }
}

pub struct AGC {
    memory: VecDeque<f32>,
    target: f32,
    linear_equalizer: f32
}

impl AGC {
    pub fn new(cap: usize, target: f32, linear_equalizer: f32) -> Self {
        let mut memory = VecDeque::with_capacity(cap);
        memory.resize(cap, target);

        AGC {
            memory,
            target,
            linear_equalizer
        }
    }
}

impl ProcessMut<f32, f32> for AGC {
    fn process(&mut self, sig: &[f32], output: &mut[f32], _: f32) {
        let mut peak: f32 = 0.0;
        let base = 12.0;
        for (x, n) in sig.iter().enumerate() {

            //let correction = (((x as f32) + 4.0)/base).min(1.0);
            let correction = ((x as f32 / base) + self.linear_equalizer).min(1.0);

            let val = *n as f32 * correction;
            peak = peak.max(val);
        }

        if peak > 0.0 {
            self.memory.pop_front();
            self.memory.push_back(peak);
        }

        let running_avg : f32 = self.memory.iter().sum::<f32>() / (self.memory.len() as f32);
        let scaler = self.target / running_avg;

        for (x, (&i, o)) in sig.iter().zip(output.iter_mut()).enumerate() {
            let correction = 1.0 - ((x as f32 / base) + self.linear_equalizer).min(1.0);

            *o = f32::min((i * scaler) , 1.0).max(0.0);
        }
    }
}

pub struct TimeAGC {
    memory: VecDeque<f32>,
    target: f32,
}

impl TimeAGC {
    pub fn new(cap: usize, target: f32) -> Self {
        let mut memory = VecDeque::with_capacity(cap);
        memory.resize(cap, target);

        TimeAGC {
            memory,
            target,
        }
    }
}

impl ProcessMut<f32, f32> for TimeAGC {
    fn process(&mut self, sig: &[f32], output: &mut[f32], _: f32) {
        let mut peak: f32 = 0.0;
        for x in sig.iter() {
            let val = x.abs();
            peak = peak.max(val);
        }

        //println!("{}", peak);

        if peak > 0.0 {
            self.memory.pop_front();
            self.memory.push_back(peak);
        }

        let running_avg : f32 = self.memory.iter().sum::<f32>() / (self.memory.len() as f32);
        let scaler = self.target / running_avg;

        for (&i, o) in sig.iter().zip(output.iter_mut()) {
            *o = f32::min(i * scaler, 1.0).max(-1.0);
        }
    }
}

pub struct PassThrough;

impl<T: Copy> Process<T,T> for PassThrough {
    fn process(&self, sig: &[T], output: &mut [T], _: f32)
    {
        output.copy_from_slice(sig);
    }
}

impl<T,U,P> ProcessMut<T,U> for P 
where P: Process<T,U> {
    fn process(&mut self, sig: &[T], output: &mut [U], p: f32)
    {
        Process::process(self, sig, output, p);
    }
}