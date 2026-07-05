#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayRuntimeVisual {
    Off,
    Booting,
    Running,
    Paused,
    Unhealthy,
    Disconnected,
}

pub(crate) struct TrayRuntimeIcons {
    pub(crate) off: tauri::image::Image<'static>,
    pub(crate) paused: tauri::image::Image<'static>,
    pub(crate) running_rgba: Vec<u8>,
    pub(crate) running_dims: (u32, u32),
    pub(crate) booting_frames: Vec<tauri::image::Image<'static>>,
}

pub(crate) fn debounced_tray_runtime_visual(
    raw_visual: TrayRuntimeVisual,
    last_non_booting: Option<TrayRuntimeVisual>,
    unhealthy_streak: &mut u8,
) -> TrayRuntimeVisual {
    const UNHEALTHY_DEBOUNCE_TICKS: u8 = 8;

    if raw_visual == TrayRuntimeVisual::Unhealthy {
        *unhealthy_streak = unhealthy_streak.saturating_add(1);
        if *unhealthy_streak < UNHEALTHY_DEBOUNCE_TICKS {
            if matches!(
                last_non_booting,
                Some(TrayRuntimeVisual::Running) | Some(TrayRuntimeVisual::Disconnected)
            ) {
                return last_non_booting.expect("checked Some above");
            }
        }
        return TrayRuntimeVisual::Unhealthy;
    }

    *unhealthy_streak = 0;
    raw_visual
}

pub(crate) fn build_tray_runtime_icons() -> anyhow::Result<TrayRuntimeIcons> {
    let decoded = image::load_from_memory_with_format(
        include_bytes!("../icons/32x32.png"),
        image::ImageFormat::Png,
    )?
    .to_rgba8();
    let width = decoded.width();
    let height = decoded.height();
    let rgba = decoded.into_vec();

    let off_rgba = add_red_badge_dot(to_grayscale_strength(&rgba, 1.0), width, height);
    // Paused intentionally has no badge: it distinguishes "user chose off"
    // from "broken and needs attention" at a glance.
    let paused_rgba = to_grayscale_strength(&rgba, 1.0);
    let booting_base = to_grayscale_strength(&rgba, 0.5);
    let booting_90 = rotate_90_cw(&booting_base, width, height);
    let booting_180 = rotate_90_cw(&booting_90, width, height);
    let booting_270 = rotate_90_cw(&booting_180, width, height);

    Ok(TrayRuntimeIcons {
        off: tauri::image::Image::new_owned(off_rgba, width, height),
        paused: tauri::image::Image::new_owned(paused_rgba, width, height),
        running_rgba: rgba,
        running_dims: (width, height),
        booting_frames: vec![
            tauri::image::Image::new_owned(booting_base, width, height),
            tauri::image::Image::new_owned(booting_90, width, height),
            tauri::image::Image::new_owned(booting_180, width, height),
            tauri::image::Image::new_owned(booting_270, width, height),
        ],
    })
}

fn to_grayscale_strength(rgba: &[u8], strength: f32) -> Vec<u8> {
    let s = strength.clamp(0.0, 1.0);
    let mut out = rgba.to_vec();
    for pixel in out.chunks_exact_mut(4) {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        let gray = 0.299 * r + 0.587 * g + 0.114 * b;
        pixel[0] = (r * (1.0 - s) + gray * s).round() as u8;
        pixel[1] = (g * (1.0 - s) + gray * s).round() as u8;
        pixel[2] = (b * (1.0 - s) + gray * s).round() as u8;
    }
    out
}

fn rotate_90_cw(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![0u8; rgba.len()];
    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let src_idx = (y * w + x) * 4;
            let dst_x = h - 1 - y;
            let dst_y = x;
            let dst_idx = (dst_y * w + dst_x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&rgba[src_idx..src_idx + 4]);
        }
    }
    out
}

fn add_red_badge_dot(mut rgba: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    let cx = w - 5;
    let cy = 5;
    let radius = 3i32;

    for y in 0..h {
        for x in 0..w {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= radius * radius {
                let idx = ((y as usize * width as usize) + x as usize) * 4;
                rgba[idx] = 217;
                rgba[idx + 1] = 76;
                rgba[idx + 2] = 76;
                rgba[idx + 3] = 255;
            }
        }
    }

    rgba
}

// Returns a (possibly wider) RGBA image with whole-dollar savings stacked
// vertically to the right of the base icon. Returns the base unchanged when
// dollars == 0.
pub(crate) fn build_running_with_savings(
    base: &[u8],
    base_w: u32,
    base_h: u32,
    dollars: u32,
) -> (Vec<u8>, u32, u32) {
    if dollars == 0 {
        return (base.to_vec(), base_w, base_h);
    }

    const CHAR_W: usize = 3;
    const CHAR_H: usize = 5;
    const H_MARGIN: usize = 2;

    let text = if dollars >= 1000 {
        format!("{}K", dollars / 1000)
    } else {
        dollars.to_string()
    };
    let chars: Vec<u8> = text.bytes().collect();
    let n = chars.len();

    let row_gap_px: usize = if n <= 2 { 2 } else { 1 };

    let available = (base_h as usize).saturating_sub(n.saturating_sub(1) * row_gap_px);
    let max_dot = if n <= 2 { 3 } else { 2 };
    let dot = (available / (n * CHAR_H)).clamp(1, max_dot);

    let col_px_w = CHAR_W * dot + H_MARGIN;
    let new_w = base_w + col_px_w as u32;
    let h = base_h as usize;
    let bw = base_w as usize;
    let nw = new_w as usize;

    let mut out = vec![0u8; nw * h * 4];

    for y in 0..h {
        let src = y * bw * 4;
        let dst = y * nw * 4;
        out[dst..dst + bw * 4].copy_from_slice(&base[src..src + bw * 4]);
    }

    let total_h = n * CHAR_H * dot + n.saturating_sub(1) * row_gap_px;
    let y0 = h.saturating_sub(total_h) / 2;
    let x0 = bw + H_MARGIN;

    for (ci, &c) in chars.iter().enumerate() {
        let glyph = pixel_char(c);
        let cy = y0 + ci * (CHAR_H * dot + row_gap_px);
        for (row, cols) in glyph.iter().enumerate() {
            for (col, &on) in cols.iter().enumerate() {
                if on == 0 {
                    continue;
                }
                for dy in 0..dot {
                    for dx in 0..dot {
                        let px = x0 + col * dot + dx;
                        let py = cy + row * dot + dy;
                        if px < nw && py < h {
                            let i = (py * nw + px) * 4;
                            out[i] = 80;
                            out[i + 1] = 210;
                            out[i + 2] = 100;
                            out[i + 3] = 240;
                        }
                    }
                }
            }
        }
    }

    (out, new_w, base_h)
}

// Each glyph is [[col0, col1, col2]; 5 rows], top to bottom.
fn pixel_char(c: u8) -> [[u8; 3]; 5] {
    match c {
        b'0' => [[1, 1, 1], [1, 0, 1], [1, 0, 1], [1, 0, 1], [1, 1, 1]],
        b'1' => [[0, 1, 0], [1, 1, 0], [0, 1, 0], [0, 1, 0], [1, 1, 1]],
        b'2' => [[1, 1, 1], [0, 0, 1], [1, 1, 1], [1, 0, 0], [1, 1, 1]],
        b'3' => [[1, 1, 1], [0, 0, 1], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
        b'4' => [[1, 0, 1], [1, 0, 1], [1, 1, 1], [0, 0, 1], [0, 0, 1]],
        b'5' => [[1, 1, 1], [1, 0, 0], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
        b'6' => [[1, 1, 1], [1, 0, 0], [1, 1, 1], [1, 0, 1], [1, 1, 1]],
        b'7' => [[1, 1, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1]],
        b'8' => [[1, 1, 1], [1, 0, 1], [1, 1, 1], [1, 0, 1], [1, 1, 1]],
        b'9' => [[1, 1, 1], [1, 0, 1], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
        b'K' => [[1, 0, 1], [1, 1, 0], [1, 0, 0], [1, 1, 0], [1, 0, 1]],
        _ => [[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0]],
    }
}
