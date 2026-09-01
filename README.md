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
#ff0000 -> L*a*b*(53.24, 80.09, 67.20)
#ff3300 -> L*a*b*(54.29, 74.87, 68.30)

Delta E (CIE76): 6.05 - perceptible at a glance
```

Colors can be given as a hex triplet (with or without the `#`) or as
comma separated `r,g,b` values:

```
$ deltae 255,0,0 255,51,0
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

Early. CIE76 is the simplest Delta E formula and is known to distort
distances in some regions of color space (saturated blues especially).
See the roadmap for what's planned.
