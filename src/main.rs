use arboard::Clipboard;
use clap::Parser;
use fastblur::gaussian_blur;
use image::{
    imageops::FilterType, io::Reader as ImageReader, DynamicImage, GenericImageView, ImageBuffer,
    Pixel, Rgb, RgbImage,
};
use std::cmp::{max, min};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input file path, defaults to clipboard
    #[arg(short, long)]
    input_path: Option<String>,

    /// output file path, defaults to clipboard
    #[arg(short, long)]
    output_path: Option<String>,
}

fn blur(image: &DynamicImage, intensity: f32) -> DynamicImage {
    let (width, height) = (image.width(), image.height());
    let pixels = image.pixels();
    let mut colors: Vec<[u8; 3]> = Vec::new();
    for (_, _, pixel) in pixels {
        colors.push(pixel.to_rgb().0);
    }
    gaussian_blur(&mut colors, width as usize, height as usize, intensity);
    let mut blurred_image_buffer = RgbImage::new(width, height);
    let mut pixel_index = 0usize;
    for y in 0..height {
        for x in 0..width {
            blurred_image_buffer.put_pixel(x, y, Rgb(colors[pixel_index]));
            pixel_index += 1;
        }
    }
    DynamicImage::ImageRgb8(blurred_image_buffer)
}
fn main() {
    let args = Args::parse();
    let image = match args.input_path {
        Some(in_path) => match ImageReader::open(in_path) {
            Ok(opened) => match opened.decode() {
                Ok(img) => img,
                Err(e) => return eprintln!("error decoding image: {e}"),
            },
            Err(e) => return eprintln!("error opening image: {e}"),
        },
        None => {
            let mut clipboard;
            match Clipboard::new() {
                Ok(cb) => clipboard = cb,
                Err(e) => return eprintln!("error accessing clipboard: {e}"),
            }
            println!("accessed clipboard");
            match clipboard.get_image() {
                Ok(img) => {
                    println!("read clipboard image");
                    match ImageBuffer::from_raw(
                        img.width.try_into().unwrap(),
                        img.height.try_into().unwrap(),
                        img.bytes.into_owned(),
                    ) {
                        Some(img) => {
                            println!("constructed clipboard image");
                            DynamicImage::ImageRgba8(img)
                        }
                        None => {
                            return eprintln!(
                                "couldn't construct clipboard image (perhaps it is empty?)"
                            )
                        }
                    }
                }
                Err(e) => return eprintln!("error reading clipboard image: {e}"),
            }
        }
    };
    _ = image.save("in.png");
    println!("creating background...");
    let (width, height) = (image.width(), image.height());
    let sqside = max(width, height);
    let bgimage = ImageBuffer::from_fn(sqside, sqside, |x, y| Rgb([255, 255, 255]));
    let factor = min(width, height);
    let resized_width = width * sqside / factor;
    let resized_height = height * sqside / factor;
    let mut bg = image.resize(resized_width, resized_height, FilterType::Triangle);
    println!("resizing done");
    bg = blur(&bg, 32.);
    println!("blurred background");
    _ = bg.save("out.png");
}
