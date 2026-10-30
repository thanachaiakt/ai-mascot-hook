//! Cut each frame strip in `assets/` into one small PNG per frame, so the running seal can
//! unpack just the frame it shows instead of holding a whole strip (megabytes) in memory.
//!
//! The strips are 8-bit palette PNGs of square frames side by side. Each becomes a blob: the
//! frame count and each frame's length (u32, little endian), then the frames' PNGs back to
//! back, all sharing the strip's palette.
//!
//! The blobs go into one file, `ai-mascot-hook.frames`, written beside the exe in
//! `target/<profile>/` (the installers ship it from there): `AMHF`, the build's id (u64),
//! the strip count (u32), each strip's offset and length (u32 pairs), then the blobs. They
//! are kept out of the exe because an exe that is nearly all compressed data looks packed to
//! antivirus models; CrowdStrike Falcon removed 0.1.9's exe, 96% frames, as it installed.
//! `$OUT_DIR/frames.rs` gives the exe the id and the strips' order, so it can tell a frames
//! file from another build.

use std::env;
use std::fs::{self, File};
use std::path::PathBuf;

const STRIPS: [&str; 12] = [
    "happy", "working", "asking", "completed",
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
    let mut blobs = Vec::with_capacity(STRIPS.len());
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
        blobs.push(blob);
    }

    // FNV-1a over every blob: equal ids mean the same frames, in the same order
    let mut id: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in blobs.iter().flatten() {
        id = (id ^ *byte as u64).wrapping_mul(0x100_0000_01b3);
    }
    let mut file = Vec::new();
    file.extend(b"AMHF");
    file.extend(id.to_le_bytes());
    file.extend((blobs.len() as u32).to_le_bytes());
    let mut at = file.len() + blobs.len() * 8;
    for blob in &blobs {
        file.extend((at as u32).to_le_bytes());
        file.extend((blob.len() as u32).to_le_bytes());
        at += blob.len();
    }
    for blob in &blobs {
        file.extend(blob);
    }
    fs::write(out.join("ai-mascot-hook.frames"), &file).expect("frames file written");
    // $OUT_DIR is target/<profile>/build/<package>-<hash>/out, and the exe lands in target/<profile>
    let profile_dir = out.ancestors().nth(3).filter(|_| out.ancestors().nth(2).and_then(|d| d.file_name()) == Some("build".as_ref()));
    if let Some(dir) = profile_dir {
        fs::write(dir.join("ai-mascot-hook.frames"), &file).expect("frames file copied beside the exe");
    }
    let names = STRIPS.map(|n| format!("{n:?}")).join(", ");
    let rs = format!("pub const FRAMES_ID: u64 = {id:#018x};\npub const STRIP_NAMES: [&str; {}] = [{names}];\n", STRIPS.len());
    fs::write(out.join("frames.rs"), rs).expect("frames.rs written");
}
