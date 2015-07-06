import argparse
import json
import os
import sys

from PIL import Image


def build_parser():
    parser = argparse.ArgumentParser(
            description='Process a bitmap font image into a usable form.')

    parser.add_argument('--font-image-in', metavar='FILE',
            required=True,
            help='path to the original font image')
    parser.add_argument('--first-char', metavar='CODE',
            default=0x21,
            help='ASCII code of the first glyph in the font')
    parser.add_argument('--space-width', metavar='SIZE',
            default=2,
            help='width of the space character in pixels')

    parser.add_argument('--font-image-out', metavar='FILE',
            required=True,
            help='path to write the processed font image')
    parser.add_argument('--font-metrics-out', metavar='FILE',
            required=True,
            help='path to write the font metrics JSON file')

    return parser


def get_glyph_boxes(img):
    # Each glyph is surrounded by a 1px margin to the top, left, and right.
    # The top-left pixel in the margin is a different color, to mark the
    # beginning of each glyph.

    assert img.mode == 'P'
    w,h = img.size

    pixels = img.getdata(0)
    # Assume top-left corner is a dot.
    dot_color = pixels[0]

    # Get the x-coordinate of each dot.
    dots = []
    for i in range(w):
        if pixels[i] == dot_color:
            dots.append(i)

    # Count the number of glyphs, then add an extra dot for the end of the
    # image.
    glyph_count = len(dots)
    dots.append(w)

    boxes = []
    for i in range(glyph_count):
        x0 = dots[i] + 1
        x1 = dots[i + 1] - 1
        y0 = 1
        y1 = h
        boxes.append((x0, y0, x1, y1))

    return boxes

def adjust_palette(img):
    """Adjust the image's palette so that the background is black and the
    glyphs are white."""

    data = img.getdata(0)
    dot_index = data[0]
    clear_index = data[1]

    palette = img.getpalette()
    for i in range(256):
        if i == dot_index:
            palette[i * 3 : i * 3 + 3] = [255, 0, 0]
        elif i == clear_index:
            palette[i * 3 : i * 3 + 3] = [0, 0, 0]
        else:
            palette[i * 3 : i * 3 + 3] = [255, 255, 255]
    img.putpalette(palette)

def build_mask(src, boxes, margin_x, margin_y):
    out_w = sum(x1 - x0 + margin_x for x0,y0,x1,y1 in boxes)
    out_h = max(y1 - y0 + margin_x for x0,y0,x1,y1 in boxes)

    out = Image.new('L', (out_w, out_h))
    x_pos = 0
    for (x0, y0, x1, y1) in boxes:
        glyph = src.crop((x0, y0, x1, y1))
        out.paste(glyph, (x_pos, 0))
        x_pos += x1 - x0 + margin_x

    return out

def build_metrics(boxes, margin_x, margin_y):
    xs = []
    y = 0
    widths = [x1 - x0 + margin_x for x0,y0,x1,y1 in boxes]
    height = max(y1 - y0 + margin_y for x0,y0,x1,y1 in boxes)

    cur_x = 0
    for w in widths:
        xs.append(cur_x)
        cur_x += w

    return {
            'xs': xs,
            'y': y,
            'widths': widths,
            'height': height,
            }

def main():
    parser = build_parser()
    args = parser.parse_args()

    img = Image.open(args.font_image_in)

    boxes = get_glyph_boxes(img)
    adjust_palette(img)
    mask = build_mask(img, boxes, 1, 1)
    metrics = build_metrics(boxes, 1, 1)


    out = Image.new('RGBA', mask.size, (0, 0, 0, 0))
    for offset in [(1, 1), (1, 0), (0, 1), (0, 0)]:
        if offset == (0, 0):
            color = (255, 255, 255, 255)
        else:
            color = (64, 64, 64, 255)
        out.paste(color, offset, mask)

    out.save(args.font_image_out)


    metrics['spacing'] = 0
    metrics['first_char'] = int(args.first_char, 0)
    metrics['space_width'] = int(args.space_width)

    with open(args.font_metrics_out, 'w') as f:
        json.dump(metrics, f)

if __name__ == '__main__':
    main()
