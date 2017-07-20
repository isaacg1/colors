# colors
Lovely programatic images

This one was generated in 63 minutes:
![Programatic image](pic10-2372992320.png)

This was generated with `cargo run --release 10`

## How it works

First, the image is seeded with n * 2 randomly colored pixels, where n is the input.

Then, a random pixel is chosen. The pixel which has already been placed whose color is closest to that pixel is located, and the new pixel is placed as close as possible to the pixel which was located. This process is repeated until the image is filled.

The pixels are not chosen purely randomly, but are instead a random ordering over a color cube with n^2 possibilities for each of R,G,B. These pixels exactly fit into the n^3 by n^3 square canvas, which is then output.

# Command line options

Put the size parameter as the first argument. Currently 1-15 are suppoered.

If you want debugging information, give a second argument. Anything is fine.

The command line interface will likely be upgraded soon.
