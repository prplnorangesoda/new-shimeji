use anyhow::{bail, Context};
use png::ColorType;
use std::{collections::HashMap, ffi::OsString};

use crate::{rgba::Rgba, shimeji::ShimejiData, xml_parser::parse};
use std::fs;

#[derive(Debug, Clone)]
pub struct AnimationData {
    pub fps: f64,
    pub frames: Vec<Frame>,
}
#[derive(Debug, Clone)]
pub struct Frame {
    pub pixels_row_major: Box<[Rgba]>,
}

pub fn create_shimeji_data_from_file_name(
    file_name: impl Into<OsString>,
) -> anyhow::Result<ShimejiData> {
    let file_name: OsString = file_name.into();
    let file = fs::File::open(file_name).context("file name passed was invalid")?;
    let data = parse(file).context("failed to parse XML data")?;

    // we have the data, create animation data in memory for the shimeji

    let mut decoded_animations = HashMap::with_capacity(data.animations.len());
    let width = data.shimeji_width;
    let height = data.shimeji_height;
    for mut animation in data.animations {
        let fps = animation.fps.unwrap_or(24.0);

        animation.frames.sort_by_key(|f| f.number);

        let mut frame_buf: Vec<Frame> = Vec::with_capacity(animation.frames.len());
        for frame in animation.frames {
            let file = fs::File::open(frame.file_path)
                .context("File specified in frame data was invalid")?;
            let decoder = png::Decoder::new(file);

            let mut reader = decoder.read_info()?;

            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader
                .next_frame(&mut buf)
                .context("could not read first png image frame")?;
            log::debug!("{info:?}");
            if info.color_type != ColorType::Rgba {
                bail!("Color type unsupported: {0:?}", info.color_type)
            }
            let size = info.buffer_size();
            if size % 4 != 0 {
                bail!("size of RGBA data buffer not divisible by 4, malformed size: {size}")
            }
            buf.truncate(size);

            let mut rgba_vec = Vec::with_capacity(size / 4);
            let mut buf_iter = buf.into_iter();
            while let Some(byte_1) = buf_iter.next() {
                let byte_2 = buf_iter.next().unwrap();
                let byte_3 = buf_iter.next().unwrap();
                let byte_4 = buf_iter.next().unwrap();

                rgba_vec.push(Rgba::new(byte_1, byte_2, byte_3, byte_4))
            }
            let bytes: Box<[Rgba]> = rgba_vec.into_boxed_slice();
            frame_buf.push(Frame {
                pixels_row_major: bytes,
            })
        }
        decoded_animations.insert(
            animation.name,
            AnimationData {
                fps,
                frames: frame_buf,
            },
        );
    }

    let ret = ShimejiData {
        name: data.name,
        animations: decoded_animations,
        height,
        width,
    };
    // log::debug!(
    //     "{:#?}",
    //     ret.animations.get("idle").unwrap().frames.first().unwrap()
    // );
    Ok(ret)
}
