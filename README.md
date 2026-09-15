# deltae

A command line tool that answers one question: how different do two colors
actually look to a human eye?

Comparing colors as raw RGB numbers is misleading. A change of 20 in the
blue channel is far less noticeable than the same change in the green
channel, and "distance" in RGB space does not line up with what people
actually perceive. This tool converts both colors into CIE L\*a\*b\*, a
color space designed so that Euclidean distance roughly tracks perceived
difference, and reports that distance as Delta E (CIE76).

Useful for things like: checking whether a rebrand's new blue is
noticeably different from the old one, deciding if two swatches in a
design system are redundant, or catching an image compression pass that
shifted colors more than intended.

## Usage

```
$ deltae '#ff0000' '#ff3300'
#ff0000 / hsl(0, 100%, 50%) -> L*a*b*(53.24, 80.09, 67.20)
#ff3300 / hsl(12, 100%, 50%) -> L*a*b*(54.29, 74.87, 68.30)

Delta E (CIE76): 6.05 - perceptible at a glance
```

Every color is echoed back as both hex and HSL regardless of how it was
entered, so you can see the same color in either notation.

Colors can be given as a hex triplet (with or without the `#`), as
comma separated `r,g,b` values, as `hsl(h, s%, l%)`, or as a CSS named
color:

```
$ deltae 255,0,0 255,51,0
$ deltae 'hsl(0, 100%, 50%)' 'hsl(12, 100%, 50%)'
$ deltae tomato orangered
```

By default the tool uses CIE76, which is simple but distorts distances
in some regions of color space (saturated blues especially). Pass
`--ciede2000` (or `-2`) to use the more accurate, more computationally
involved CIEDE2000 formula instead:

```
$ deltae --ciede2000 '#ff0000' '#ff3300'
```

Pass `--format json` to get a single machine readable JSON object instead
of the text report, useful for piping into scripts or other tools:

```
$ deltae --format json '#ff0000' '#ff3300'
{"color1":{"input":"#ff0000","hex":"ff0000","rgb":[255,0,0],"hsl":{"h":0.00,"s":100.00,"l":50.00},"lab":{"l":53.24,"a":80.09,"b":67.20}},"color2":{"input":"#ff3300","hex":"ff3300","rgb":[255,51,0],"hsl":{"h":12.00,"s":100.00,"l":50.00},"lab":{"l":54.29,"a":74.87,"b":68.30}},"formula":"cie76","delta_e":6.05,"interpretation":"perceptible at a glance"}
```

Delta E is roughly interpreted as:

| Delta E | Meaning |
|---------|---------|
| < 1     | not perceptible |
| 1 - 2   | perceptible on close inspection |
| 2 - 10  | perceptible at a glance |
| 10 - 50 | colors are more similar than different |
| > 50    | colors are essentially opposite |

## Building

Standard library only, no dependencies:

```
cargo build --release
```

## Status

Early. CIE76, CIEDE2000, named CSS colors, HSL input/output, and JSON
output are implemented; batch mode is still planned.
