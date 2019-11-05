use rustfft::num_complex::Complex;

pub trait Window {
    fn window(&self, data: &[f32], output: &mut [Complex<f32>]);
}


fn sinc_window_inner(coeff: &[f32], data: &[f32], output: &mut [Complex<f32>]) {
    use std::f32::consts::PI;
    let len = data.len();
    for (n, x) in data.iter().enumerate() {
        output[n] = (x * coeff.iter().enumerate().map(|(k,&a)| {
            let kf = k as f32;
            let n = n as f32;
            (-1.0f32).powi(k as i32) * a * ((2.0*PI*kf*n)/(len - 1) as f32).cos()
        }).sum::<f32>()).into();
    }
}

pub struct SincWindow<'a> {
    pub coeff: &'a [f32]
}

impl<'a> Window for SincWindow<'a> {
    fn window(&self, data: &[f32], output: &mut [Complex<f32>]) {
        sinc_window_inner(self.coeff, data, output);
    }
}

pub struct NutallWindow;
impl Window for NutallWindow {
    fn window(&self, data: &[f32], output: &mut [Complex<f32>]) {
        sinc_window_inner(&[0.3635819, 0.4891775, 0.1365995, 0.0106411], data, output);
    }
}

pub fn f_to_bin(fft_size: usize, fs: f32, f: f32) -> usize {
    (f * (fft_size as f32 / fs)) as usize
}