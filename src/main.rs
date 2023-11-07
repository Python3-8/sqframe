use arboard::Clipboard;
use clap::Parser;
use image::{
    imageops::{blur, resize, FilterType},
    io::Reader as ImageReader,
    DynamicImage, ImageBuffer, Rgb,
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
    let (width, height) = (image.width(), image.height());
    let sqside = max(width, height);
    let bgimage = ImageBuffer::from_fn(sqside, sqside, |x, y| Rgb([255, 255, 255]));
    let factor = min(width, height);
    let resized_width = width * sqside / factor;
    let resized_height = height * sqside / factor;
    let mut resized_blurred = image.clone();
    resized_blurred = DynamicImage::ImageRgba8(resize(
        &resized_blurred,
        resized_width,
        resized_height,
        FilterType::Triangle,
    ));
    println!("created resized background");
    resized_blurred = DynamicImage::ImageRgba8(blur(&resized_blurred, 10f32));
    println!("blurred background");
    _ = resized_blurred.save("out.png");
}
