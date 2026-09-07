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
    let mut positional: Vec<&String> = Vec::new();
    for arg in &args[1..] {
        if is_help(arg) {
            print_usage(&args[0]);
            return ExitCode::SUCCESS;
        }
        if arg == "--ciede2000" || arg == "-2" {
            use_ciede2000 = true;
        } else {
            positional.push(arg);
        }
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

    println!("{} -> L*a*b*({:.2}, {:.2}, {:.2})", fmt_rgb(a), lab_a.0, lab_a.1, lab_a.2);
    println!("{} -> L*a*b*({:.2}, {:.2}, {:.2})", fmt_rgb(b), lab_b.0, lab_b.1, lab_b.2);
    println!();

    if use_ciede2000 {
        let de = delta_e2000(lab_a, lab_b);
        println!("Delta E (CIEDE2000): {:.2} - {}", de, interpret(de));
    } else {
        let de = delta_e76(lab_a, lab_b);
        println!("Delta E (CIE76): {:.2} - {}", de, interpret(de));
    }

    ExitCode::SUCCESS
}

fn is_help(s: &str) -> bool {
    s == "-h" || s == "--help"
}

fn print_usage(bin: &str) {
    eprintln!("usage: {bin} [--ciede2000] <color1> <color2>");
    eprintln!();
    eprintln!("colors may be given as a hex triplet or as r,g,b (each 0-255):");
    eprintln!("  {bin} '#ff0000' '#ff3300'");
    eprintln!("  {bin} 255,0,0 255,51,0");
    eprintln!();
    eprintln!("--ciede2000 (or -2) uses the CIEDE2000 formula instead of CIE76.");
    eprintln!("It corrects for known distortions in CIE76, particularly around");
    eprintln!("saturated blues, at the cost of a lot more arithmetic.");
}

// Accepts "#rrggbb", "rrggbb", or "r,g,b".
fn parse_color(input: &str) -> Result<Rgb, String> {
    let s = input.trim();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return parse_hex(s);
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

    Err(format!("unrecognized color format: '{input}'"))
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
