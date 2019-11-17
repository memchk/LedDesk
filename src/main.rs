use crossbeam::channel;
use rustfft::num_complex::Complex;
use rustfft::*;
use serialport::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;
use structopt::StructOpt;

mod color;
mod math;
mod process;
mod strided_chunks;

use color::RGB;
use math::{NutallWindow, Window};
use process::{Process, ProcessMut};

#[derive(StructOpt, Debug, Clone)]
struct Args {
    #[structopt(long = "nobar")]
    no_bar: bool,
    #[structopt(long = "port", default_value = "COM3")]
    com_port: String,
    #[structopt(default_value = "0.04", long = "decay")]
    decay_time: f32,
    #[structopt(default_value = "1", long = "fftscale")]
    fftscale: f32,
    #[structopt(default_value = "8000", long = "mf")]
    mf: f32,
    #[structopt(long = "fft", default_value = "1536")]
    fft_size: usize,
    #[structopt(long = "exp", default_value = "1.0")]
    exp: f32,
    #[structopt(long = "prescale", default_value = "2.0")]
    prescale: f32,
    #[structopt(long = "color", default_value = "FF00FF")]
    color: String,
}

const NUM_LEDS: usize = 82;

fn main() {
    let args = Args::from_args();

    let color = RGB::from_hex(&args.color).saturate();
    println!("{:?}", color);
    let (audio_sender, audio_recv) = channel::bounded(args.fft_size);
    let _ = std::thread::spawn(move || audio_thread(audio_sender));

    let (led_sender, led_recv) = channel::bounded(NUM_LEDS);
    let cloned_args = args.clone();
    let _ = std::thread::spawn(move || fft_thread(audio_recv, led_sender, cloned_args));


    // Main thread takes care of sending the data down UART to micro for display.
    let mut led_port = serialport::open_with_settings(
        &args.com_port,
        &SerialPortSettings {
            baud_rate: 500000,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(100),
        },
    )
    .unwrap();

    let mut buf = Vec::with_capacity(50 * 3 + 50 * 3 + NUM_LEDS * 4);
    let mut frame = Vec::with_capacity(NUM_LEDS);
    loop {
        // Setup a channel iterator, then fill the framebuffer after clearing it.
        let frame_iter = led_recv
            .iter()
            .take(NUM_LEDS)
            .enumerate()
            .map(|(_, b)| b * color);

        frame.clear();
        frame.extend(frame_iter);

        let bar_e = 0;
        let l_bar: Vec<_> = std::iter::repeat(bar_e)
            .take(50)
            .map(|e| color::RGB::new(e, e, e))
            .collect();

        let r_bar: Vec<_> = std::iter::repeat(bar_e)
            .take(50)
            .map(|e| color::RGB::new(e, e, e))
            .collect();

        buf.extend_from_slice(b"Ada");
        buf.extend_from_slice(&[0x01, 0x06, 0x52]);

        buf.extend(frame.iter().rev().flat_map(color::RGB::as_slice));
        buf.extend(l_bar.iter().flat_map(color::RGB::as_slice));
        // Bottom
        buf.extend(frame.iter().flat_map(color::RGB::as_slice));
        buf.extend(r_bar.iter().flat_map(color::RGB::as_slice));

        let _ = led_port.write_all(&buf);
        let _ = led_port.flush();
        buf.clear();
    }
}

// FFT processing.
fn fft_thread(
    audio_reciever: channel::Receiver<(f32, f32)>,
    led_sender: channel::Sender<f32>,
    args: Args,
) {
    let fft_size = args.fft_size;
    let fft_nyquist = fft_size / 2;

    // Allocate our buffers ahead of time, dynamic allocations in a tight loop are bad.
    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(fft_size);
    let mut sample_vec = VecDeque::with_capacity(fft_size);
    let mut windowed_samples = vec![Complex::default(); fft_size];
    let mut fft_data = vec![Complex::default(); fft_size];
    let mut f32_scratch = vec![0.0; fft_size];
    let mut leds = vec![0.0; NUM_LEDS];

    // Only interested in the first half since the is real data, and because
    // nyquist is a problem.
    let mut fft_energy = vec![0.0; fft_nyquist];

    // Calculate the overlap to faciliate a pseudo-welch's method.
    let overlap = fft_size / 2;

    // Setup the principal decay engine.
    let mut fft_decay = process::ExpDecay::new(
        fft_nyquist,
        44100.0 / fft_nyquist as f32,
        args.decay_time,
        1.0,
    );

    // Setup the Frequency -> LED mapper.
    let led_map = process::PageLog::new(fft_size, 44100.0, args.mf, NUM_LEDS);

    loop {
        // Create a running buffer, dropping and consuming `overlap` amounts of data each time, except for initial fill.
        // fill from the audio thread.
        let drain_amount = std::cmp::min(sample_vec.len(), overlap);
        sample_vec.drain(..drain_amount);

        let sample_iterator = audio_reciever
            .iter()
            .take(fft_size - sample_vec.len())
            .map(|(l, r)| (l + r) / 2.0);
        
        sample_vec.extend(sample_iterator);

        // Copy into a continous buffer since a dequeue is represented as two slices.
        let (sample_left, sample_right) = sample_vec.as_slices();
        let left_len = sample_left.len();
        let (scratch_l, scratch_r) = f32_scratch.split_at_mut(left_len);
        scratch_l.copy_from_slice(sample_left);
        scratch_r.copy_from_slice(sample_right);

        // Window the data to prevent spectral contamination, then compute the FFT.
        NutallWindow.window(&f32_scratch[..], &mut windowed_samples[..]);
        fft.process(&mut windowed_samples[..], &mut fft_data[..]);

        for (c, a) in fft_data.iter().zip(fft_energy.iter_mut()) {
            // First get the amplitude, normalize by dividing by the nyquist bin, then apply a prescaler.
            let amplitude = args.prescale * c.to_polar().0 / fft_nyquist as f32;
            // Apply a power function to require at args.exp > 1.0 a more powerful signal to trigger an led activation.
            *a = amplitude.powf(args.exp);
        }

        // Process the decay, then calculate the log magnitude from there.
        fft_decay.process(&mut fft_energy[..], &mut f32_scratch[..], 1.0);

        // Floor the function if nessiary, then apply a postscaler.
        for (&i, o) in f32_scratch.iter().zip(fft_energy.iter_mut()) {
            if i < 0.001 {
                *o = 0.0;
            } else {
                //*o = args.fftscale * i.sqrt();
                *o = args.fftscale * i;
            }
        }

        // Map to leds, and send to the main thread for display.
        led_map.process(&fft_energy[..], &mut leds[..], 1.0);
        for &l in &leds {
            let _ = led_sender.try_send(l);
        }
    }
}

// Principal audio thread. This pulls data from windows WASAPI and streams it into a crossbeam
// channel for further processing.
fn audio_thread(audio_channel: channel::Sender<(f32, f32)>) {
    let device = cpal::default_output_device().expect("Could not get output device.");
    let format = device
        .default_output_format()
        .expect("Could not get output format.");
    let event_loop = cpal::EventLoop::new();
    let id = event_loop
        .build_input_stream(&device, &format, true)
        .unwrap();

    event_loop.play_stream(id);
    event_loop.run(|_, data| match data {
        cpal::StreamData::Input {
            buffer: cpal::UnknownTypeInputBuffer::F32(buffer),
        } => {
            let lr_pairs = buffer.chunks_exact(2).map(|x| (x[0], x[1]));
            for pair in lr_pairs {
                let _ = audio_channel.try_send(pair);
            }
        }
        _ => {}
    });
}
