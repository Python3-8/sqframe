use arboard::{Clipboard, ImageData};
use clap::Parser;
use fastblur::gaussian_blur;
use image::{
    imageops::FilterType, io::Reader as ImageReader, DynamicImage, GenericImageView, ImageBuffer,
    Pixel, Rgb, RgbImage,
};
use std::{
    borrow::Cow,
    cmp::{max, min},
    env,
    fs::rename,
    io,
    io::Write,
    path::Path,
};

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

fn get_colors(image: &DynamicImage) -> Vec<[u8; 3]> {
    let pixels = image.pixels();
    let mut colors: Vec<[u8; 3]> = Vec::new();
    for (_, _, pixel) in pixels {
        colors.push(pixel.to_rgb().0);
    }
    colors
}

fn get_colors_alpha(image: &DynamicImage) -> Vec<[u8; 4]> {
    let pixels = image.pixels();
    let mut colors: Vec<[u8; 4]> = Vec::new();
    for (_, _, pixel) in pixels {
        colors.push(pixel.0);
    }
    colors
}

fn blur(image: &DynamicImage, intensity: f32) -> DynamicImage {
    let (width, height) = (image.width(), image.height());
    let mut colors = get_colors(&image);
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

enum ConfirmResult {
    Continue,
    Stop,
    IOError(io::Error),
}

fn confirm(msg: String) -> ConfirmResult {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut resp = String::new();
    loop {
        resp.clear();
        print!("{msg}");
        _ = stdout.flush();
        match stdin.read_line(&mut resp) {
            Ok(_) => {}
            Err(e) => return ConfirmResult::IOError(e),
        };
        resp = resp.trim().to_lowercase();
        if ["y".to_string(), "yes".to_string()].contains(&resp) {
            return ConfirmResult::Continue;
        }
        if ["n".to_string(), "no".to_string()].contains(&resp) {
            return ConfirmResult::Stop;
        }
    }
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
    println!("creating background...");
    let (width, height) = (image.width(), image.height());
    let sqside = max(width, height);
    let factor = min(width, height);
    let resized_width = width * sqside / factor;
    let resized_height = height * sqside / factor;
    let mut bg = image.resize(resized_width, resized_height, FilterType::Triangle);
    println!("upscale: done");
    bg = bg.crop(
        (resized_width - sqside) / 2,
        (resized_height - sqside) / 2,
        sqside,
        sqside,
    );
    println!("square crop: done");
    bg = blur(&bg, 16.);
    println!("blur background: done");
    println!("background created");
    println!("constructing final image");
    let x_rng = ((sqside - image.width()) / 2)..((sqside + image.width()) / 2);
    let y_rng = ((sqside - image.height()) / 2)..((sqside + image.height()) / 2);
    let mut final_image = RgbImage::new(sqside, sqside);
    let mut orig_pixels = image.pixels();
    for y in 0..sqside {
        for x in 0..sqside {
            if x_rng.contains(&x) && y_rng.contains(&y) {
                match orig_pixels.next() {
                    Some(px) => final_image.put_pixel(x, y, px.2.to_rgb()),
                    _ => {}
                }
            } else {
                final_image.put_pixel(x, y, bg.get_pixel(x, y).to_rgb());
            }
        }
    }
    let final_image = DynamicImage::ImageRgb8(final_image);
    println!("done!");
    let temp_dir = env::temp_dir();
    match args.output_path {
        Some(out_path) => {
            let output_path = out_path.clone();
            let path = Path::new(&output_path);
            if path.is_dir() || path.is_symlink() {
                return eprintln!(
                    "{output_path} is a directory or a symbolic link, cannot proceed"
                );
            }
            if path.is_file() {
                match confirm(format!(
                    "{output_path} is an existing file. replace? [y/n]: "
                )) {
                    ConfirmResult::Continue => {
                        let backup_path = temp_dir.join(Path::new("BACKUP"));
                        match rename(&path, &backup_path) {
                            Ok(_) => {
                                println!("original file backed up to: {}", backup_path.display())
                            }
                            Err(e) => eprintln!("error backing up original file: {e}"),
                        }
                    }
                    ConfirmResult::Stop => {
                        return println!("please rerun with a different output path");
                    }
                    ConfirmResult::IOError(e) => {
                        return eprintln!("error while trying to read stdin: {e}");
                    }
                }
            }
            match final_image.save(&output_path) {
                Ok(_) => return println!("saved image!"),
                Err(e) => return eprintln!("error saving image: {e}"),
            };
        }
        None => {
            let backup_path = temp_dir.join(Path::new("clipboard.png"));
            match image.save(&backup_path) {
                Ok(_) => {
                    println!(
                        "backed up original clipboard content to: {}",
                        backup_path.display()
                    );
                }
                Err(e) => eprintln!("error backing up original clipboard content: {e}"),
            }
            let bytes = get_colors_alpha(&final_image).join(&[][..]);
            let image_data = ImageData {
                width: sqside as usize,
                height: sqside as usize,
                bytes: Cow::from(&bytes),
            };
            let mut clipboard;
            match Clipboard::new() {
                Ok(cb) => clipboard = cb,
                Err(e) => return eprintln!("error accessing clipboard: {e}"),
            }
            match clipboard.set_image(image_data) {
                Ok(_) => return println!("edited image copied to clipboard!"),
                Err(e) => return eprintln!("error copying edited image to clipboard: {e}"),
            }
        }
    }
}
