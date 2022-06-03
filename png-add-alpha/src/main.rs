use std::{env, fs, io::BufWriter};
use tempfile::NamedTempFile;

const BYTES_PER_RGBA_PIXEL: usize = 4;
const DEFAULT_OPAQUE_ALPHA: u8 = u8::MAX;

// TODO: Write tests

// PNG RGB -> RGBA, in-place converter
fn main() {
    let path_to_png = env::args()
        .skip(1)
        .nth(0)
        .expect("Expected one argument, input PNG file to normalize color type to RGBA.");
    let path_to_png =
        fs::canonicalize(path_to_png).expect("Could not canonicalize path to input PNG file.");

    let (img_buf, width, height, bit_depth) = {
        let mut decoder = png::Decoder::new(
            fs::File::open(&path_to_png).expect("Could not open input PNG file."),
        );
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
            bit_depth,
            ..
        } = info;
        assert!(
            (color_type == png::ColorType::Rgba) || (color_type == png::ColorType::Rgb),
            "Unexpected input PNG file color format, expected RGB or RGBA"
        );

        let size = reader.output_buffer_size();
        let mut input_img_buf = vec![0; size];
        reader
            .next_frame(&mut input_img_buf)
            .expect("Could not read image data from input PNG file.");

        let img_buf = match color_type {
            png::ColorType::Rgb => {
                let pixels: usize = (width * height) as _;
                let new_raw_bytes = pixels * BYTES_PER_RGBA_PIXEL;
                assert!(size < new_raw_bytes);

                // TODO: Speed this up
                // - Try using SIMD simd<u8, 64>, scatter/gather
                let mut tmp_buf = vec![DEFAULT_OPAQUE_ALPHA; new_raw_bytes];
                for i in 0..pixels {
                    tmp_buf[i * 4] = input_img_buf[i * 3];
                    tmp_buf[i * 4 + 1] = input_img_buf[i * 3 + 1];
                    tmp_buf[i * 4 + 2] = input_img_buf[i * 3 + 2];
                }
                tmp_buf
            }
            png::ColorType::Rgba => input_img_buf,
            _ => panic!("Unexpected input PNG file color format, expected RGB or RGBA"),
        };
        (img_buf, width, height, bit_depth)
    };

    let out_file = NamedTempFile::new().expect("Unable to create temporary output file");
    {
        let mut encoder = png::Encoder::new(BufWriter::new(&out_file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(bit_depth);
        encoder
            .write_header()
            .expect("Unable to write image header into output PNG file")
            .write_image_data(&img_buf)
            .expect("Unable to write image data into output PNG file");
    }

    // Replace the original input PNG file
    out_file
        .persist(&path_to_png)
        .expect("Could not persist output PNG file");
}
