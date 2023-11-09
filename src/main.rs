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
    time::{SystemTime, UNIX_EPOCH},
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

/// Returns a vector containing arrays of length 3 (R, G, B) corresponding to the pixels in the image
fn get_colors(image: &DynamicImage) -> Vec<[u8; 3]> {
    let pixels = image.pixels();
    let mut colors: Vec<[u8; 3]> = Vec::new();
    for (_, _, pixel) in pixels {
        colors.push(pixel.to_rgb().0);
    }
    colors
}

/// Returns a vector containing arrays of length 4 (R, G, B, A) corresponding to the pixels in the image
fn get_colors_alpha(image: &DynamicImage) -> Vec<[u8; 4]> {
    let pixels = image.pixels();
    let mut colors: Vec<[u8; 4]> = Vec::new();
    for (_, _, pixel) in pixels {
        colors.push(pixel.0);
    }
    colors
}

/// Returns a blurred (Gaussian blur) copy of the image, with `intensity` being the blur radius
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

/// Returns an image with `fg` overlaid on `bg`, assuming that `fg` can fit into `bg`
fn overlay(bg: &DynamicImage, fg: &DynamicImage) -> DynamicImage {
    let (bg_width, bg_height) = (bg.width(), bg.height());
    let x_rng = ((bg_width - fg.width()) / 2)..((bg_width + fg.width()) / 2);
    let y_rng = ((bg_height - fg.height()) / 2)..((bg_height + fg.height()) / 2);
    let mut final_image = RgbImage::new(bg_width, bg_height);
    let mut orig_pixels = fg.pixels();
    for y in 0..bg_height {
        for x in 0..bg_width {
            if x_rng.contains(&x) && y_rng.contains(&y) {
                match orig_pixels.next() {
                    Some(px) => final_image.put_pixel(x, y, px.2.to_rgb()),
                    _ => {}
                }
                continue;
            }
            final_image.put_pixel(x, y, bg.get_pixel(x, y).to_rgb());
        }
    }
    DynamicImage::ImageRgb8(final_image)
}

/// Returns a hyphen (`"-"`) followed by the current timestamp in milliseconds if successful, otherwise an empty string
fn get_timestamp_suffix() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => return format!("-{}", duration.as_millis()),
        Err(_) => return "".to_string(),
    }
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
                Err(e) => return eprintln!("error decoding image: {e:?}"),
            },
            Err(e) => return eprintln!("error opening image: {e:?}"),
        },
        None => match Clipboard::new() {
            Ok(mut clipboard) => {
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
                    Err(e) => return eprintln!("error reading clipboard image: {e:?}"),
                }
            }
            Err(e) => return eprintln!("error accessing clipboard: {e:?}"),
        },
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
    println!("gaussian blur: done");
    println!("background created");
    println!("constructing final image");
    let final_image = overlay(&bg, &image);
    println!("done!");
    let temp_dir = env::temp_dir();
    match args.output_path {
        Some(out_path) => {
            let output_path = Path::new(&out_path);
            if output_path.is_dir() || output_path.is_symlink() {
                return eprintln!(
                    "{:?} is a directory or a symbolic link, cannot proceed",
                    output_path.display()
                );
            }
            if output_path.is_file() {
                match confirm(format!(
                    "{:?} is an existing file. replace? [y/n]: ",
                    output_path.display()
                )) {
                    ConfirmResult::Continue => {
                        let backup_path =
                            temp_dir.join(Path::new(&format!("BACKUP{}", get_timestamp_suffix())));
                        match rename(&output_path, &backup_path) {
                            Ok(_) => {
                                println!(
                                    "original file at {:?} backed up to: {:?}",
                                    output_path.display(),
                                    backup_path.display()
                                )
                            }
                            Err(e) => eprintln!(
                                "error backing up original file at {:?}: {e:?}",
                                output_path.display(),
                            ),
                        }
                    }
                    ConfirmResult::Stop => {
                        return println!("please rerun with a different output path");
                    }
                    ConfirmResult::IOError(e) => {
                        return eprintln!("error while trying to read stdin: {e:?}");
                    }
                }
            }
            match final_image.save(output_path) {
                Ok(_) => return println!("saved image to {:?}!", output_path.display()),
                Err(e) => {
                    return eprintln!("error saving image to {:?}: {e:?}", output_path.display())
                }
            };
        }
        None => {
            let backup_path = temp_dir.join(Path::new(&format!(
                "CLIPBOARD{}.png",
                get_timestamp_suffix()
            )));
            match image.save(&backup_path) {
                Ok(_) => {
                    println!(
                        "backed up original clipboard content to: {:?}",
                        backup_path.display()
                    );
                }
                Err(e) => eprintln!("error backing up original clipboard content: {e:?}"),
            }
            let bytes = get_colors_alpha(&final_image).join(&[][..]);
            let image_data = ImageData {
                width: sqside as usize,
                height: sqside as usize,
                bytes: Cow::from(&bytes),
            };
            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_image(image_data) {
                    Ok(_) => return println!("edited image copied to clipboard!"),
                    Err(e) => return eprintln!("error copying edited image to clipboard: {e:?}"),
                },
                Err(e) => return eprintln!("error accessing clipboard: {e:?}"),
            }
        }
    }
}
