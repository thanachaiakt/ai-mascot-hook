//! Cut each frame strip in `assets/` into one small PNG per frame, so the running seal can
//! unpack just the frame it shows instead of holding a whole strip (megabytes) in memory.
//!
//! The strips are 8-bit palette PNGs of square frames side by side. Each becomes
//! `$OUT_DIR/<name>.frames`: the frame count and each frame's length (u32, little endian),
//! then the frames' PNGs back to back, all sharing the strip's palette.

use std::env;
use std::fs::{self, File};
use std::path::PathBuf;

const STRIPS: [&str; 12] = [
    "loading", "working", "asking", "completed",
    "thinking", "active", "error", "slide", "break-time", "toy-car", "ball", "salmon",
];

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    // the exe's own icon (Explorer, shortcuts, the installer). winresource is a Windows-host
    // build dependency, so a Mac host compiles this block out.
    #[cfg(windows)]
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        // embedding needs a resource compiler (the Windows SDK's rc.exe); a build box without one
        // still gets a working exe, just an iconless one. The release build must not skip it.
        if let Err(e) = res.compile() {
            assert!(env::var_os("CI").is_none(), "exe icon: {e}");
        }
    }
    for name in STRIPS {
        let path = format!("assets/{name}.png");
        println!("cargo:rerun-if-changed={path}");
        let mut decoder = png::Decoder::new(File::open(&path).expect("frame strip"));
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().expect("frame strip header");
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("frame strip decodes");
        assert!(info.color_type == png::ColorType::Indexed && info.bit_depth == png::BitDepth::Eight, "{path} must be an 8-bit palette PNG");
        let (w, h) = (info.width as usize, info.height as usize);
        let palette = reader.info().palette.as_ref().expect("palette").to_vec();
        let trns = reader.info().trns.as_ref().map(|t| t.to_vec());

        let frames = w / h;
        let mut pngs = Vec::with_capacity(frames);
        for f in 0..frames {
            let mut px = Vec::with_capacity(h * h);
            for y in 0..h {
                px.extend_from_slice(&buf[y * info.line_size + f * h..][..h]);
            }
            let mut png = Vec::new();
            let mut encoder = png::Encoder::new(&mut png, h as u32, h as u32);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Best);
            encoder.set_palette(palette.clone());
            if let Some(trns) = &trns {
                encoder.set_trns(trns.clone());
            }
            let mut writer = encoder.write_header().expect("frame header");
            writer.write_image_data(&px).expect("frame encodes");
            writer.finish().expect("frame finishes");
            pngs.push(png);
        }

        let mut blob = Vec::new();
        blob.extend((frames as u32).to_le_bytes());
        for png in &pngs {
            blob.extend((png.len() as u32).to_le_bytes());
        }
        for png in &pngs {
            blob.extend(png);
        }
        fs::write(out.join(format!("{name}.frames")), blob).expect("frames written");
    }
}
