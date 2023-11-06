use arboard::Clipboard;
use clap::Parser;
use image::{io::Reader as ImageReader, DynamicImage, ImageBuffer};

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
                    match ImageBuffer::from_raw(
                        img.width.try_into().unwrap(),
                        img.height.try_into().unwrap(),
                        img.bytes.into_owned(),
                    ) {
                        Some(img) => DynamicImage::ImageRgba8(img),
                        None => {
                            return eprintln!("couldn't construct image (perhaps it is empty?)")
                        }
                    }
                }
                Err(e) => return eprintln!("error reading image: {e}"),
            }
        }
    };
    image.save("cb.png").expect("bruhh couldn't even save");
}
