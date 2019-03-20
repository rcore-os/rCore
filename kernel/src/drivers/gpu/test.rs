use core::slice;

// from Wikipedia
fn hsv_to_rgb(h: u32, s: f32, v: f32) -> (f32, f32, f32) {
    let hi = (h / 60) % 6;
    let f = (h % 60) as f32 / 60.0;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match hi {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        5 => (v, p, q),
        _ => unreachable!(),
    }
}

pub fn mandelbrot(width: u32, height: u32, frame_buffer: *mut u32) {
    let size = width * height * 4;
    let frame_buffer_data =
        unsafe { slice::from_raw_parts_mut(frame_buffer as *mut u32, (size / 4) as usize) };
    for x in 0..width {
        for y in 0..height {
            let index = y * width + x;
            let scale = 5e-3;
            let xx = (x as f32 - width as f32 / 2.0) * scale;
            let yy = (y as f32 - height as f32 / 2.0) * scale;
            let mut re = xx as f32;
            let mut im = yy as f32;
            let mut iter: u32 = 0;
            loop {
                iter = iter + 1;
                let new_re = re * re - im * im + xx as f32;
                let new_im = re * im * 2.0 + yy as f32;
                if new_re * new_re + new_im * new_im > 1e3 {
                    break;
                }
                re = new_re;
                im = new_im;

                if iter == 60 {
                    break;
                }
            }
            iter = iter * 6;
            let (r, g, b) = hsv_to_rgb(iter, 1.0, 0.5);
            let rr = (r * 256.0) as u32;
            let gg = (g * 256.0) as u32;
            let bb = (b * 256.0) as u32;
            let color = (bb << 16) | (gg << 8) | rr;
            frame_buffer_data[index as usize] = color;
        }
        println!("working on x {}/{}", x, width);
    }
}
