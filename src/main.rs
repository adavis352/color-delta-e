// deltae: answers one question - how different do these two colors look
// to a human eye? Measured as CIE76 Delta E in the L*a*b* color space,
// because Euclidean distance in raw RGB does not track perception at all
// (e.g. green channel differences look much smaller than blue ones).

use std::env;
use std::process::ExitCode;

type Rgb = (u8, u8, u8);
type Lab = (f64, f64, f64);

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    let mut use_ciede2000 = false;
    let mut format = OutputFormat::Text;
    let mut positional: Vec<&String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if is_help(arg) {
            print_usage(&args[0]);
            return ExitCode::SUCCESS;
        }
        if arg == "--ciede2000" || arg == "-2" {
            use_ciede2000 = true;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            match OutputFormat::parse(value) {
                Some(f) => format = f,
                None => {
                    eprintln!("error: unrecognized format '{value}' (expected 'text' or 'json')");
                    return ExitCode::FAILURE;
                }
            }
        } else if arg == "--format" {
            i += 1;
            let value = match args.get(i) {
                Some(v) => v,
                None => {
                    eprintln!("error: --format requires a value ('text' or 'json')");
                    return ExitCode::FAILURE;
                }
            };
            match OutputFormat::parse(value) {
                Some(f) => format = f,
                None => {
                    eprintln!("error: unrecognized format '{value}' (expected 'text' or 'json')");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            positional.push(arg);
        }
        i += 1;
    }

    if positional.len() != 2 {
        print_usage(&args[0]);
        return ExitCode::FAILURE;
    }

    let a = match parse_color(positional[0]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let b = match parse_color(positional[1]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let lab_a = rgb_to_lab(a);
    let lab_b = rgb_to_lab(b);

    let formula_name = if use_ciede2000 { "ciede2000" } else { "cie76" };
    let de = if use_ciede2000 { delta_e2000(lab_a, lab_b) } else { delta_e76(lab_a, lab_b) };

    match format {
        OutputFormat::Text => {
            println!(
                "{} / {} -> L*a*b*({:.2}, {:.2}, {:.2})",
                fmt_rgb(a),
                fmt_hsl(a),
                lab_a.0,
                lab_a.1,
                lab_a.2
            );
            println!(
                "{} / {} -> L*a*b*({:.2}, {:.2}, {:.2})",
                fmt_rgb(b),
                fmt_hsl(b),
                lab_b.0,
                lab_b.1,
                lab_b.2
            );
            println!();
            let label = if use_ciede2000 { "CIEDE2000" } else { "CIE76" };
            println!("Delta E ({label}): {:.2} - {}", de, interpret(de));
        }
        OutputFormat::Json => {
            println!("{}", to_json(positional[0], a, lab_a, positional[1], b, lab_b, formula_name, de));
        }
    }

    ExitCode::SUCCESS
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    fn parse(s: &str) -> Option<OutputFormat> {
        match s {
            "text" => Some(OutputFormat::Text),
            "json" => Some(OutputFormat::Json),
            _ => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn to_json(
    input_a: &str,
    rgb_a: Rgb,
    lab_a: Lab,
    input_b: &str,
    rgb_b: Rgb,
    lab_b: Lab,
    formula: &str,
    de: f64,
) -> String {
    format!(
        concat!(
            "{{\"color1\":{},\"color2\":{},",
            "\"formula\":\"{}\",\"delta_e\":{:.2},\"interpretation\":\"{}\"}}"
        ),
        color_json(input_a, rgb_a, lab_a),
        color_json(input_b, rgb_b, lab_b),
        formula,
        de,
        interpret(de),
    )
}

fn color_json(input: &str, rgb @ (r, g, b): Rgb, (l, la, lb): Lab) -> String {
    let (h, s, sl) = rgb_to_hsl(rgb);
    format!(
        concat!(
            "{{\"input\":\"{}\",\"hex\":\"{:02x}{:02x}{:02x}\",\"rgb\":[{},{},{}],",
            "\"hsl\":{{\"h\":{:.2},\"s\":{:.2},\"l\":{:.2}}},",
            "\"lab\":{{\"l\":{:.2},\"a\":{:.2},\"b\":{:.2}}}}}"
        ),
        json_escape(input),
        r,
        g,
        b,
        r,
        g,
        b,
        h,
        s,
        sl,
        l,
        la,
        lb,
    )
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn is_help(s: &str) -> bool {
    s == "-h" || s == "--help"
}

fn print_usage(bin: &str) {
    eprintln!("usage: {bin} [--ciede2000] [--format text|json] <color1> <color2>");
    eprintln!();
    eprintln!("colors may be given as a hex triplet, as r,g,b (each 0-255), as");
    eprintln!("hsl(h, s%, l%), or as a CSS named color:");
    eprintln!("  {bin} '#ff0000' '#ff3300'");
    eprintln!("  {bin} 255,0,0 255,51,0");
    eprintln!("  {bin} 'hsl(0, 100%, 50%)' 'hsl(16, 100%, 50%)'");
    eprintln!("  {bin} tomato orangered");
    eprintln!();
    eprintln!("--ciede2000 (or -2) uses the CIEDE2000 formula instead of CIE76.");
    eprintln!("It corrects for known distortions in CIE76, particularly around");
    eprintln!("saturated blues, at the cost of a lot more arithmetic.");
    eprintln!();
    eprintln!("--format json prints a single machine readable JSON object instead");
    eprintln!("of the default text report.");
}

// Accepts "#rrggbb", "rrggbb", "r,g,b", "hsl(h, s%, l%)", or a CSS named
// color like "rebeccapurple".
fn parse_color(input: &str) -> Result<Rgb, String> {
    let s = input.trim();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return parse_hex(s);
    }
    let lower = s.to_ascii_lowercase();
    if let Some(inner) = lower.strip_prefix("hsl(").and_then(|rest| rest.strip_suffix(')')) {
        return parse_hsl(inner);
    }
    if s.contains(',') {
        let parts: Vec<&str> = s.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            return Err(format!("expected 3 comma separated channels, got {}", parts.len()));
        }
        let mut channels = [0u8; 3];
        for (i, p) in parts.iter().enumerate() {
            channels[i] = p
                .parse::<u16>()
                .ok()
                .filter(|v| *v <= 255)
                .ok_or_else(|| format!("channel out of range 0-255: '{p}'"))?
                as u8;
        }
        return Ok((channels[0], channels[1], channels[2]));
    }
    if let Some(rgb) = named_color(s) {
        return Ok(rgb);
    }

    Err(format!("unrecognized color format: '{input}'"))
}

// The CSS Color Module Level 4 extended keyword list.
fn named_color(name: &str) -> Option<Rgb> {
    let rgb = match name.to_ascii_lowercase().as_str() {
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "black" => (0, 0, 0),
        "blanchedalmond" => (255, 235, 205),
        "blue" => (0, 0, 255),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "cyan" => (0, 255, 255),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "fuchsia" => (255, 0, 255),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "gray" | "grey" => (128, 128, 128),
        "green" => (0, 128, 0),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "magenta" => (255, 0, 255),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 205, 170),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orange" => (255, 165, 0),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "purple" => (128, 0, 128),
        "rebeccapurple" => (102, 51, 153),
        "red" => (255, 0, 0),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "white" => (255, 255, 255),
        "whitesmoke" => (245, 245, 245),
        "yellow" => (255, 255, 0),
        "yellowgreen" => (154, 205, 50),
        _ => return None,
    };
    Some(rgb)
}

// Parses the inside of "hsl(...)": "h, s%, l%" with optional whitespace.
// The '%' on s and l is accepted with or without it, since it's easy to
// drop when typing these by hand.
fn parse_hsl(inner: &str) -> Result<Rgb, String> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!("expected hsl(h, s%, l%), got {} components", parts.len()));
    }
    let h = parts[0].parse::<f64>().map_err(|_| format!("invalid hue: '{}'", parts[0]))?;
    let s = parse_percent("saturation", parts[1])?;
    let l = parse_percent("lightness", parts[2])?;
    if !(0.0..=100.0).contains(&s) || !(0.0..=100.0).contains(&l) {
        return Err("saturation and lightness must be between 0% and 100%".to_string());
    }
    Ok(hsl_to_rgb(h.rem_euclid(360.0), s, l))
}

fn parse_percent(label: &str, s: &str) -> Result<f64, String> {
    let trimmed = s.strip_suffix('%').unwrap_or(s);
    trimmed.parse::<f64>().map_err(|_| format!("invalid {label}: '{s}'"))
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let s = s / 100.0;
    let l = l / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to_u8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

fn rgb_to_hsl((r, g, b): Rgb) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let delta = max - min;

    if delta == 0.0 {
        return (0.0, 0.0, l * 100.0);
    }

    let s = if l < 0.5 { delta / (max + min) } else { delta / (2.0 - max - min) };
    let mut h = if max == r {
        ((g - b) / delta) % 6.0
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    (h, s * 100.0, l * 100.0)
}

fn fmt_hsl(rgb: Rgb) -> String {
    let (h, s, l) = rgb_to_hsl(rgb);
    format!("hsl({h:.0}, {s:.0}%, {l:.0}%)")
}

fn parse_hex(hex: &str) -> Result<Rgb, String> {
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("invalid hex color: '{hex}'"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    Ok((r, g, b))
}

fn fmt_rgb((r, g, b): Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

// sRGB (gamma encoded, 0-255) -> linear light -> CIE XYZ (D65) -> CIE L*a*b*.
fn rgb_to_lab(rgb: Rgb) -> Lab {
    xyz_to_lab(rgb_to_xyz(rgb))
}

fn srgb_channel_to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn rgb_to_xyz((r, g, b): Rgb) -> (f64, f64, f64) {
    let rl = srgb_channel_to_linear(r);
    let gl = srgb_channel_to_linear(g);
    let bl = srgb_channel_to_linear(b);

    // sRGB -> XYZ matrix for the D65 reference white.
    let x = rl * 0.4124564 + gl * 0.3575761 + bl * 0.1804375;
    let y = rl * 0.2126729 + gl * 0.7151522 + bl * 0.0721750;
    let z = rl * 0.0193339 + gl * 0.1191920 + bl * 0.9503041;

    (x * 100.0, y * 100.0, z * 100.0)
}

fn xyz_to_lab((x, y, z): (f64, f64, f64)) -> Lab {
    // D65 reference white, 2 degree observer.
    const XN: f64 = 95.0489;
    const YN: f64 = 100.0;
    const ZN: f64 = 108.8840;

    let fx = lab_f(x / XN);
    let fy = lab_f(y / YN);
    let fz = lab_f(z / ZN);

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    (l, a, b)
}

fn lab_f(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    if t > DELTA * DELTA * DELTA {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

fn delta_e76(a: Lab, b: Lab) -> f64 {
    let dl = a.0 - b.0;
    let da = a.1 - b.1;
    let db = a.2 - b.2;
    (dl * dl + da * da + db * db).sqrt()
}

// CIEDE2000: corrects CIE76 for perceptual non-uniformities in L*a*b*,
// mainly the way it overstates differences among saturated blues and
// understates them for low-chroma neutrals. Formula per Sharma, Wu &
// Dalal (2005), which is the version implementations are usually checked
// against since the original CIE text has a couple of ambiguous spots.
fn delta_e2000(lab1: Lab, lab2: Lab) -> f64 {
    let (l1, a1, b1) = lab1;
    let (l2, a2, b2) = lab2;

    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar7 = ((c1 + c2) / 2.0).powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25f64.powi(7))).sqrt());

    let a1p = a1 * (1.0 + g);
    let a2p = a2 * (1.0 + g);
    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();

    let hp = |ap: f64, b: f64| -> f64 {
        if ap == 0.0 && b == 0.0 { 0.0 } else { b.atan2(ap).to_degrees().rem_euclid(360.0) }
    };
    let h1p = hp(a1p, b1);
    let h2p = hp(a2p, b2);

    let dl_p = l2 - l1;
    let dc_p = c2p - c1p;
    let dh_p = if c1p * c2p == 0.0 {
        0.0
    } else {
        let diff = h2p - h1p;
        if diff.abs() <= 180.0 {
            diff
        } else if diff > 180.0 {
            diff - 360.0
        } else {
            diff + 360.0
        }
    };
    let dh_p_big = 2.0 * (c1p * c2p).sqrt() * (dh_p.to_radians() / 2.0).sin();

    let l_bar_p = (l1 + l2) / 2.0;
    let c_bar_p = (c1p + c2p) / 2.0;
    let h_bar_p = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * (h_bar_p - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar_p).to_radians().cos()
        + 0.32 * (3.0 * h_bar_p + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar_p - 63.0).to_radians().cos();

    let delta_theta = 30.0 * (-((h_bar_p - 275.0) / 25.0).powi(2)).exp();
    let c_bar_p7 = c_bar_p.powi(7);
    let r_c = 2.0 * (c_bar_p7 / (c_bar_p7 + 25f64.powi(7))).sqrt();
    let r_t = -r_c * (2.0 * delta_theta.to_radians()).sin();

    let s_l = 1.0 + (0.015 * (l_bar_p - 50.0).powi(2)) / (20.0 + (l_bar_p - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_p;
    let s_h = 1.0 + 0.015 * c_bar_p * t;

    let term_l = dl_p / s_l;
    let term_c = dc_p / s_c;
    let term_h = dh_p_big / s_h;

    (term_l * term_l + term_c * term_c + term_h * term_h + r_t * term_c * term_h).sqrt()
}

// Thresholds are the commonly cited rules of thumb for CIE76 Delta E.
// They're a decent approximation for CIEDE2000 too since both formulas
// are scaled to roughly the same range, but treat them as a rule of
// thumb either way, not a precise perceptual boundary.
fn interpret(de: f64) -> &'static str {
    match de {
        d if d < 1.0 => "not perceptible to the human eye",
        d if d < 2.0 => "perceptible only through close observation",
        d if d < 10.0 => "perceptible at a glance",
        d if d < 50.0 => "colors are more similar than different",
        _ => "colors are essentially opposite",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_with_and_without_hash() {
        assert_eq!(parse_color("#ff0000").unwrap(), (255, 0, 0));
        assert_eq!(parse_color("00ff00").unwrap(), (0, 255, 0));
    }

    #[test]
    fn parses_comma_separated_rgb() {
        assert_eq!(parse_color("255, 0, 0").unwrap(), (255, 0, 0));
    }

    #[test]
    fn rejects_out_of_range_channel() {
        assert!(parse_color("300,0,0").is_err());
    }

    #[test]
    fn parses_named_css_colors_case_insensitively() {
        assert_eq!(parse_color("tomato").unwrap(), (255, 99, 71));
        assert_eq!(parse_color("Tomato").unwrap(), (255, 99, 71));
        assert_eq!(parse_color("REBECCAPURPLE").unwrap(), (102, 51, 153));
    }

    #[test]
    fn rejects_unknown_named_color() {
        assert!(parse_color("notacolor").is_err());
    }

    #[test]
    fn parses_hsl_input() {
        assert_eq!(parse_color("hsl(0, 100%, 50%)").unwrap(), (255, 0, 0));
        assert_eq!(parse_color("HSL(120, 100%, 50%)").unwrap(), (0, 255, 0));
        assert_eq!(parse_color("hsl(0, 0%, 100%)").unwrap(), (255, 255, 255));
        assert_eq!(parse_color("hsl(0, 0%, 0%)").unwrap(), (0, 0, 0));
    }

    #[test]
    fn hsl_percent_sign_is_optional() {
        assert_eq!(parse_color("hsl(0, 100, 50)").unwrap(), parse_color("hsl(0, 100%, 50%)").unwrap());
    }

    #[test]
    fn rejects_out_of_range_hsl_percent() {
        assert!(parse_color("hsl(0, 150%, 50%)").is_err());
    }

    #[test]
    fn rejects_malformed_hsl() {
        assert!(parse_color("hsl(0, 100%)").is_err());
        assert!(parse_color("hsl(x, 100%, 50%)").is_err());
    }

    #[test]
    fn rgb_hsl_round_trips_for_primaries() {
        for rgb in [(255, 0, 0), (0, 255, 0), (0, 0, 255), (128, 64, 32), (10, 200, 150)] {
            let (h, s, l) = rgb_to_hsl(rgb);
            let back = hsl_to_rgb(h, s, l);
            let close = |a: u8, b: u8| (a as i16 - b as i16).abs() <= 1;
            assert!(
                close(rgb.0, back.0) && close(rgb.1, back.1) && close(rgb.2, back.2),
                "{rgb:?} -> hsl({h}, {s}, {l}) -> {back:?}"
            );
        }
    }

    #[test]
    fn grayscale_has_zero_saturation() {
        let (_, s, _) = rgb_to_hsl((128, 128, 128));
        assert_eq!(s, 0.0);
    }

    #[test]
    fn format_parses_known_values_only() {
        assert!(OutputFormat::parse("text") == Some(OutputFormat::Text));
        assert!(OutputFormat::parse("json") == Some(OutputFormat::Json));
        assert!(OutputFormat::parse("xml").is_none());
    }

    #[test]
    fn json_escape_handles_quotes_and_backslashes() {
        assert_eq!(json_escape(r#"say "hi"\now"#), r#"say \"hi\"\\now"#);
    }

    #[test]
    fn color_json_reports_hex_rgb_and_lab() {
        let lab = rgb_to_lab((255, 0, 0));
        let json = color_json("#ff0000", (255, 0, 0), lab);
        assert!(json.contains("\"input\":\"#ff0000\""));
        assert!(json.contains("\"hex\":\"ff0000\""));
        assert!(json.contains("\"rgb\":[255,0,0]"));
        assert!(json.contains("\"hsl\":{\"h\":0.00,\"s\":100.00,\"l\":50.00}"));
        assert!(json.contains("\"l\":53.24"));
    }

    #[test]
    fn to_json_is_well_formed_and_matches_inputs() {
        let lab_a = rgb_to_lab((255, 0, 0));
        let lab_b = rgb_to_lab((255, 51, 0));
        let de = delta_e76(lab_a, lab_b);
        let json = to_json("#ff0000", (255, 0, 0), lab_a, "#ff3300", (255, 51, 0), lab_b, "cie76", de);
        assert!(json.starts_with('{') && json.ends_with('}'));
        assert!(json.contains("\"formula\":\"cie76\""));
        assert!(json.contains(&format!("\"delta_e\":{:.2}", de)));
    }

    #[test]
    fn identical_colors_have_zero_delta() {
        let lab = rgb_to_lab((120, 45, 200));
        assert!(delta_e76(lab, lab) < 1e-9);
    }

    #[test]
    fn black_and_white_are_maximally_different() {
        let black = rgb_to_lab((0, 0, 0));
        let white = rgb_to_lab((255, 255, 255));
        assert!(delta_e76(black, white) > 90.0);
    }

    #[test]
    fn identical_colors_have_zero_ciede2000_delta() {
        let lab = rgb_to_lab((120, 45, 200));
        assert!(delta_e2000(lab, lab) < 1e-9);
    }

    // Reference pairs from Sharma, Wu & Dalal's published CIEDE2000 test
    // data (2005), used by most implementations to check against the
    // formula's special cases (near-zero chroma, hue averaging across
    // the 0/360 boundary).
    #[test]
    fn matches_sharma_reference_values() {
        let cases: [((f64, f64, f64), (f64, f64, f64), f64); 4] = [
            ((50.0, 2.6772, -79.7751), (50.0, 0.0, -82.7485), 2.0425),
            ((50.0, -1.3802, -84.2814), (50.0, 0.0, -82.7485), 1.0000),
            ((50.0, 2.5, 0.0), (73.0, 25.0, -18.0), 27.1492),
            ((16.2550, -0.7315, -0.5406), (16.0819, -0.0499, -0.0389), 0.6377),
        ];
        for (a, b, expected) in cases {
            let got = delta_e2000(a, b);
            assert!(
                (got - expected).abs() < 0.001,
                "expected {expected}, got {got} for {a:?} vs {b:?}"
            );
        }
    }
}
