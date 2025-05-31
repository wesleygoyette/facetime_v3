use clap::builder::Str;
use opencv::{
    core::{AlgorithmHint, Mat, Size},
    imgproc::{COLOR_BGR2GRAY, INTER_LINEAR, cvt_color, resize},
    prelude::*,
};

const ASCII_CHARS: &[char] = &[' ', '.', ',', ':', ';', '+', '*', '?', '%', 'S', '#', '@'];

pub struct AsciiConverter {
    width: i32,
    height: i32,
}

impl AsciiConverter {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn frame_to_ascii(&self, frame: &Mat) -> opencv::Result<String> {
        let mut gray = Mat::default();
        cvt_color(
            frame,
            &mut gray,
            COLOR_BGR2GRAY,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;

        let mut resized = Mat::default();
        let size = Size::new(self.width, self.height);
        resize(&gray, &mut resized, size, 0.0, 0.0, INTER_LINEAR)?;

        let mut ascii_art = String::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let pixel_value = *resized.at_2d::<u8>(y, self.width - 1 - x)?;
                let ascii_index = (pixel_value as usize * (ASCII_CHARS.len() - 1)) / 255;
                ascii_art.push(ASCII_CHARS[ascii_index]);
            }
            ascii_art.push('\n');
        }

        Ok(ascii_art)
    }

    pub fn merge_ascii_frames_side_by_side(frame1: String, frame2: String) -> String {
        let lines1: Vec<&str> = frame1.lines().collect();
        let lines2: Vec<&str> = frame2.lines().collect();
    
        let max_lines = lines1.len().max(lines2.len());
        let mut merged = String::new();
    
        for i in 0..max_lines {
            let line1 = lines1.get(i).copied().unwrap_or("");
            let line2 = lines2.get(i).copied().unwrap_or("");
            merged.push_str(line1);
            merged.push(' '); // optional separator
            merged.push_str(line2);
            merged.push('\n');
        }
    
        merged
    }
}
