use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn main() {
    let png_path = Path::new("assets/brand/app_icon.png");
    let ico_path = Path::new("assets/brand/app.ico");

    if png_path.exists() {
        let mut png_file = File::open(png_path).expect("failed to open app_icon.png");
        let mut png_data = Vec::new();
        png_file
            .read_to_end(&mut png_data)
            .expect("failed to read png");

        let mut ico_file = File::create(ico_path).expect("failed to create app.ico");

        // Write ICO Header
        ico_file.write_all(&[0, 0]).unwrap(); // Reserved
        ico_file.write_all(&[1, 0]).unwrap(); // Type (1 for ICO)
        ico_file.write_all(&[1, 0]).unwrap(); // Count (1 image)

        // Write Directory Entry
        ico_file.write_all(&[0]).unwrap(); // Width (0 for 256)
        ico_file.write_all(&[0]).unwrap(); // Height (0 for 256)
        ico_file.write_all(&[0]).unwrap(); // Color count (0)
        ico_file.write_all(&[0]).unwrap(); // Reserved
        ico_file.write_all(&[1, 0]).unwrap(); // Color planes (1)
        ico_file.write_all(&[32, 0]).unwrap(); // Bits per pixel (32)

        // Image size (4 bytes, little endian)
        let size = png_data.len() as u32;
        ico_file.write_all(&size.to_le_bytes()).unwrap();

        // Image offset (4 bytes, little endian) - Header (6) + DirEntry (16) = 22
        ico_file.write_all(&22u32.to_le_bytes()).unwrap();

        // Write PNG data
        ico_file.write_all(&png_data).unwrap();
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        if ico_path.exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/brand/app.ico");
            res.compile().expect("failed to compile winres resource");
        }
    }
}
