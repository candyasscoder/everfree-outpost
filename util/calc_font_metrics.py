import json
import os
import sys

from PIL import Image


def main(image_file, first_code):
    img = Image.open(image_file)
    w, h = img.size

    assert img.mode == 'P'
    pixels = img.getdata(0)
    # Assume top-left corner is a dot.
    dot_color = pixels[0]

    dots = []
    for i in range(w):
        if pixels[i] == dot_color:
            dots.append(i)
    glyph_count = len(dots)
    dots.append(w)

    starts = []
    widths = []
    for i in range(glyph_count):
        starts.append(dots[i] + 1)
        widths.append(dots[i + 1] - dots[i] - 2)

    j = {
            'first': first_code,
            'xs': starts,
            'widths': widths,
            'y': 1,
            'height': h - 1,
            }
    json.dump(j, sys.stdout)


if __name__ == '__main__':
    image_file, first_code_str = sys.argv[1:]
    first_code = int(first_code_str, 0)
    main(image_file, first_code)
