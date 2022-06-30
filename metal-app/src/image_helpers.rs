use std::{fs, path::Path};

pub const BYTES_PER_PIXEL: u32 = 4; // Assumed to be 4-component (ex. RGBA)

pub fn read_png_pixel_bytes_into<P: AsRef<Path>>(
    path_to_png: P,
    mut buffer: &mut Vec<u8>,
) -> (usize, (u32, u32)) {
    let mut decoder =
        png::Decoder::new(fs::File::open(&path_to_png).expect("Could not open input PNG file."));
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().expect("Could not read input PNG file.");
    let info = reader.info();
    assert!(
        info.trns.is_none(),
        "input PNG file contains unsupported tRNS"
    );
    let &png::Info {
        width,
        height,
        color_type,
        ..
    } = info;

    assert!(
        (color_type == png::ColorType::Rgba),
        "Unexpected input PNG file color format, expected RGB or RGBA"
    );

    let size = reader.output_buffer_size();
    buffer.resize(size, 0);
    reader
        .next_frame(&mut buffer)
        .expect("Could not read image data from input PNG file.");
    (size, (width, height))
}
