use arboard::{Clipboard, ImageData};
use clap::Parser;
use colored::Colorize;
use fastblur::gaussian_blur;
use image::{
    imageops::FilterType, io::Reader as ImageReader, DynamicImage, GenericImageView, ImageBuffer,
    Pixel, Rgb, RgbImage,
};
use std::{
    borrow::Cow,
    cmp::{max, min},
    env, fs, io,
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

/// A tool to create a square frame with a blurred background for any image, to match the aspect ratio 1:1
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input file path, defaults to clipboard
    #[arg(short, long)]
    input_path: Option<String>,

    /// Output file path, defaults to clipboard
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
        Err(_) => return String::from(""),
    }
}

enum ConfirmResult {
    Continue,
    Stop,
    IOError(io::Error),
}

/// Prompts the user with a message, expecting "yes", or "no" and returns a `ConfirmResult`
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
        if [String::from("y"), String::from("yes")].contains(&resp) {
            return ConfirmResult::Continue;
        }
        if [String::from("n"), String::from("no")].contains(&resp) {
            return ConfirmResult::Stop;
        }
    }
}

fn raise(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg.bold().red());
    process::exit(1)
}

fn open_image_from_path(input_path: &str) -> DynamicImage {
    match ImageReader::open(input_path) {
        Ok(opened) => {
            println!("Opened image from {input_path:?}");
            match opened.decode() {
                Ok(img) => {
                    println!("Decoded image");
                    img
                }
                Err(e) => raise(&format!("Could not decode image: {e:?}")),
            }
        }
        Err(e) => raise(&format!("Could not open image: {e:?}")),
    }
}

fn open_image_from_clipboard() -> DynamicImage {
    match Clipboard::new() {
        Ok(mut clipboard) => {
            println!("Accessed clipboard");
            match clipboard.get_image() {
                Ok(img) => {
                    println!("Read clipboard image");
                    match ImageBuffer::from_raw(
                        img.width.try_into().unwrap(),
                        img.height.try_into().unwrap(),
                        img.bytes.into_owned(),
                    ) {
                        Some(img) => {
                            println!("Constructed clipboard image");
                            DynamicImage::ImageRgba8(img)
                        }
                        None => raise("Could not construct clipboard image"),
                    }
                }
                Err(e) => raise(&format!("Could not read clipboard image: {e:?}")),
            }
        }
        Err(e) => raise(&format!("Error accessing clipboard: {e:?}")),
    }
}

fn open_image(input_path: Option<String>) -> DynamicImage {
    match input_path {
        Some(in_path) => open_image_from_path(&in_path),
        None => open_image_from_clipboard(),
    }
}

fn save_image_to_path(image: DynamicImage, output_path: &Path, temp_dir: PathBuf) {
    if output_path.is_dir() || output_path.is_symlink() {
        raise(&format!(
            "{:?} is a directory or a symbolic link, cannot proceed",
            output_path.display()
        ))
    }
    if output_path.is_file() {
        match confirm(format!(
            "{:?} is an existing file. replace? [y/n]: ",
            output_path.display()
        )) {
            ConfirmResult::Continue => {
                let backup_path =
                    temp_dir.join(Path::new(&format!("BACKUP{}", get_timestamp_suffix())));
                match fs::rename(&output_path, &backup_path) {
                    Ok(_) => {
                        println!(
                            "Original file at {:?} backed up to: {:?}",
                            output_path.display(),
                            backup_path.display()
                        )
                    }
                    Err(e) => raise(&format!(
                        "Could not back up original file at {:?}: {e:?}",
                        output_path.display()
                    )),
                }
            }
            ConfirmResult::Stop => {
                println!("Please rerun with a different output path, or without an output path (to copy the result to the clipboard)");
                process::exit(0)
            }
            ConfirmResult::IOError(e) => raise(&format!("Error while trying to read stdin: {e:?}")),
        }
    }
    match image.save(output_path) {
        Ok(_) => return println!("Saved image to {:?}!", output_path.display()),
        Err(e) => raise(&format!(
            "Could not save image to {:?}: {e:?}",
            output_path.display()
        )),
    }
}

fn save_image_to_clipboard(image: DynamicImage) {
    match confirm(String::from(
        "Overwrite clipboard content with edited image? [y/n]: ",
    )) {
        ConfirmResult::Continue => {
            let bytes = get_colors_alpha(&image).join(&[][..]);
            let image_data = ImageData {
                width: image.width() as usize,
                height: image.height() as usize,
                bytes: Cow::from(&bytes),
            };
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.set_image(image_data) {
                        Ok(_) => return println!("Edited image copied to clipboard!"),
                        Err(e) => {
                            raise(&format!("Could not copy edited image to clipboard: {e:?}"))
                        }
                    };
                }
                Err(e) => raise(&format!("Could not access clipboard: {e:?}")),
            }
        }
        ConfirmResult::Stop => {
            println!("Please rerun with the clipboard content backed up, or with an output path specified (see '--help')");
            process::exit(0)
        }
        ConfirmResult::IOError(e) => raise(&format!("Could not read stdin: {e:?}")),
    }
}

fn save_image(image: DynamicImage, output_path: Option<String>) {
    let temp_dir = env::temp_dir();
    match output_path {
        Some(out_path) => save_image_to_path(image, Path::new(&out_path), temp_dir),
        None => save_image_to_clipboard(image),
    }
}

pub fn run(args: Args) {
    let image = open_image(args.input_path);
    println!("Creating blurred background...");
    let (width, height) = (image.width(), image.height());
    let sqside = max(width, height);
    let factor = min(width, height);
    let resized_width = width * sqside / factor;
    let resized_height = height * sqside / factor;
    let mut bg = image.resize(resized_width, resized_height, FilterType::Triangle);
    println!("Upscale: done");
    bg = bg.crop(
        (resized_width - sqside) / 2,
        (resized_height - sqside) / 2,
        sqside,
        sqside,
    );
    println!("Square crop: done");
    bg = blur(&bg, 16.);
    println!("Gaussian blur: done");
    println!("Background created");
    println!("Constructing final image...");
    let final_image = overlay(&bg, &image);
    println!("Done!");
    save_image(final_image, args.output_path);
}
