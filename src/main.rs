//use parking_lot::Mutex;
//use std::sync::Arc;

use crossbeam::channel;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::{FFTplanner, FFT};
//use sample::{signal, Frame, Sample, Signal, ToFrameSliceMut};
use serialport::prelude::*;
use std::iter;
use structopt::StructOpt;
use std::thread::sleep;
use std::time::{Duration, Instant};

mod color;
mod math;
mod process;

use math::*;
use process::{Process, ProcessMut};

const NUM_LEDS: usize = 82;

#[derive(StructOpt, Debug, Clone)]
struct Args {
    // Log is the default
    #[structopt(long = "log", group = "scale")]
    log: bool,
    #[structopt(long = "linear", group = "scale")]
    linear: bool,
    // #[structopt(long = "tonal", group = "scale")]
    // tonal: bool,
    #[structopt(long = "split")]
    spilt_channel: bool,

    #[structopt(default_value="0.04", long="decay")]
    decay_time: f32,
    #[structopt(default_value = "1536")]
    fft_size: usize,
}

fn main() {
    let args = Args::from_args();
    let device = cpal::default_output_device().expect("Failed to get default output device");
    let format = device
        .default_output_format()
        .expect("Failed to get default output format");
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop
        .build_input_stream(&device, &format, true)
        .expect("Failed to build loopback stream");
    event_loop.play_stream(stream_id);

    let num_leds = if args.spilt_channel {
        NUM_LEDS / 2
    } else {
        NUM_LEDS
    };

    let mut led_port = serialport::open_with_settings(
        "COM6",
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

    let fft_size = args.fft_size;

    let (audio_s, fft_r) = channel::bounded(fft_size * 2);

    // Get Audio samples and send them in a queue.
    std::thread::spawn(move || {
        event_loop.run(|_, data| {
            match data {
                cpal::StreamData::Input {
                    buffer: cpal::UnknownTypeInputBuffer::F32(buffer),
                } => {
                    let lr_pairs = buffer.chunks_exact(2).map(|s| (s[0], s[1]));
                    for x in lr_pairs {
                        let _ = audio_s.try_send(x);
                    }
                }
                _ => (),
            }
        });
    });

    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(fft_size);

    let fft_thread = fft.clone();
    let (fft_s, led_r) = channel::bounded(fft_size * 10);
    let (fft_ps, led_pr) = channel::bounded(fft_size);

    let args_thread = args.clone();

    let post = process::PageLog::new(fft_size as u32, 44100.0, 4800.0, num_leds);

    //let think_time = Duration::from_millis(((fft_size as f32 / 44100.0 ) * 1010.0) as u64);
    let think_time = Duration::from_millis(33);

    std::thread::spawn(move || {
        let mut fft_ol: Vec<Complex<_>> = vec![Zero::zero(); fft_size];
        let mut fft_or: Vec<Complex<_>> = vec![Zero::zero(); fft_size];

        let mut fft_al: Vec<_> = vec![0.0f32; fft_size / 2];
        let mut fft_ar: Vec<_> = vec![0.0f32; fft_size / 2];

        let mut led_l: Vec<_> = vec![0.0; num_leds];
        let mut led_r: Vec<_> = vec![0.0; num_leds];

        let m_e = max_energy(fft_size);

        let mut decay = process::ExpDecay::new(fft_size, 44100.0 / fft_size as f32, args_thread.decay_time, 1.0);

        let mut l_dec = process::ExpDecay::new(1, 44100.0 / fft_size as f32, args_thread.decay_time, m_e);
        let mut r_dec = process::ExpDecay::new(1, 44100.0 / fft_size as f32, args_thread.decay_time, m_e);

        let mut fft_scratch = vec![0.0f32; fft_size];
        loop {
            let loop_begin = Instant::now();

            let (mut sl, sr): (Vec<_>, Vec<_>) = fft_r
                .try_iter()
                .chain(iter::repeat((0.0, 0.0)))
                .take(fft_size)
                .enumerate()
                .map(|(i, (l, r))| {
                    (l * hann(i, fft_size), r * hann(i, fft_size))
                    //(l, r)
                }).unzip();

            let l_e_p = &[spectral_energy(&sl)];
            let r_e_p = &[spectral_energy(&sr)];

            let l_e = &mut [0.0f32];
            let r_e = &mut [0.0f32];
            l_dec.process(l_e_p, l_e, 1.0);
            r_dec.process(r_e_p, r_e, 1.0);

            let l_e = (f32::min(l_e[0] / (m_e * 0.5), 1.0) * 255.0) as u8;
            let r_e = (f32::min(r_e[0] / (m_e * 0.5), 1.0) * 255.0) as u8;

            let _ = fft_ps.try_send((l_e, r_e));

            let scale = 1.0;

            if args_thread.spilt_channel {
                let (mut il, mut ir) : (Vec<_>, Vec<_>) = sl.iter().zip(sr.iter()).map(|(&l, &r)| (Complex::new(l, 0.0), Complex::new(r, 0.0))).unzip();
                fft_thread.process(&mut il, &mut fft_ol);
                fft_thread.process(&mut ir, &mut fft_or);

                fft_amp(&fft_ol, &mut fft_al, m_e);
                fft_amp(&fft_or, &mut fft_ar, m_e);
                post.process(&fft_al, &mut led_l, scale);
                post.process(&fft_ar, &mut led_r, scale);
            } else {
                mix(&mut sl, &sr);
                let mut im : Vec<_> = sl.iter().map(|&l| Complex::new(l, 0.0)).collect();
                fft_thread.process(&mut im, &mut fft_ol);

                fft_amp(&fft_ol, &mut fft_al, m_e);
                
                decay.process(&fft_al, &mut fft_scratch, scale);
                std::mem::swap(&mut fft_al, &mut fft_scratch);
                post.process(&fft_al, &mut led_l, scale);
            }


            for (&l, &r) in led_l.iter().zip(led_r.iter()) {
                let _ = fft_s.try_send((l, r));
            }

            if let Some(sleep_time) = think_time.checked_sub(loop_begin.elapsed()) {
                sleep(sleep_time);
            }

        }
    });

    use std::io::prelude::*;

    let mut buf = Vec::with_capacity(50 * 3 + 50*3 + NUM_LEDS * 4);
    loop {
        let (l_frame, r_frame): (Vec<_>, Vec<_>) = led_r
            .iter()
            .take(num_leds)
            .enumerate()
            .map(|(i, (l, r))| {
                // let color_base = if i < (num_leds / 3) {
                //     color::RGB::new(1, 0, 0)
                // } else if i < (2 * num_leds) / 3 {
                //     color::RGB::new(0, 1, 0)
                // } else {
                //     color::RGB::new(0, 0, 1)
                // };
                
                let color_base = color::RGB::new(185, 0, 255);
                (color_base * l, color_base * r)
            })
            .unzip();

        let (l_e, r_e) = led_pr.recv().unwrap();

        let l_bar: Vec<_> = std::iter::repeat(l_e)
            .take(50)
            .map(|e| color::RGB::new(e, e, e))
            .collect();

        let r_bar: Vec<_> = std::iter::repeat(r_e)
            .take(50)
            .map(|e| color::RGB::new(e, e, e))
            .collect();

        buf.extend_from_slice(b"Ada");
        buf.extend_from_slice(&[0x01, 0x06, 0x52]);

        if args.spilt_channel {
            // Top of Desk
            buf.extend(r_frame.iter().rev().flat_map(color::RGB::as_slice));
            buf.extend(l_frame.iter().flat_map(color::RGB::as_slice));
            buf.extend(l_bar.iter().flat_map(color::RGB::as_slice));

            // Bottom
            buf.extend(l_frame.iter().rev().flat_map(color::RGB::as_slice));
            buf.extend(r_frame.iter().flat_map(color::RGB::as_slice));
            buf.extend(r_bar.iter().flat_map(color::RGB::as_slice));
        } else {
            buf.extend(l_frame.iter().rev().flat_map(color::RGB::as_slice));
            buf.extend(l_bar.iter().flat_map(color::RGB::as_slice));
            // Bottom
            buf.extend(l_frame.iter().flat_map(color::RGB::as_slice));
            buf.extend(r_bar.iter().flat_map(color::RGB::as_slice));
        }
        
        let _ = led_port.write_all(&buf);
        let _ = led_port.flush();
        buf.clear();
    }
}
