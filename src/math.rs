use rustfft::num_complex::Complex;

#[inline]
pub fn hann(i: usize, n: usize) -> f32 {
    use std::f32::consts::PI;
    //(1.0 - ((2.0 * PI * i as f32) / (n as f32 - 1.0)).cos()) / 2.0
    ((PI * i as f32) / n as f32).sin().powi(2)
}

#[inline]
pub fn max_energy(fft_size: usize) -> f32 {
    (0..fft_size/2).fold(0.0, |acc, x| acc + hann(x, fft_size).powi(2))
}

#[inline]
pub fn freq_to_bin(f: f32, size: f32, sample_rate: f32) -> usize {
    (f / (sample_rate / size)) as usize
}

#[inline]
pub fn fft_amp(i: &[Complex<f32>], o: &mut [f32], m_e: f32) {
    i.iter()
        .zip(o.iter_mut())
        .for_each(|(&i, o)| *o = 2.0 * (i.to_polar().0.powi(2)) / m_e);
}

#[inline]
pub fn spectral_energy(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0, |acc, x| acc + x.powi(2))
}

#[inline]
pub fn mix(left: &mut [f32], right: &[f32]) {
    left.iter_mut()
        .zip(right.iter())
        .for_each(|(l, &o)| *l = (*l + o) / 2.0)
}
