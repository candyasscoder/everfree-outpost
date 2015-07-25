import os
import sys
from PIL import Image

path = sys.argv[1]
base, ext = os.path.splitext(path)

transparent_path = base + '-transparent' + ext
if os.path.exists(transparent_path):
    img = Image.open(transparent_path)
else:
    print('Making background transparent')
    img = Image.open(path).convert('RGBA')
    w, h = img.size
    border = img.getpixel((0, 0))
    background = img.getpixel((1, 1))

    for y in range(h):
        if y % 10 == 0:
            print('Fixing background (%4d / %4d)' % (y, h))
        for x in range(w):
            if img.getpixel((x, y)) in (border, background):
                img.putpixel((x, y), (0, 0, 0, 0))

    img.save(transparent_path)

box = lambda x, y, w: (x * 96, y * 96, (x + w) * 96, (y + 1) * 96)

for d in range(5):
    print('Generating part %d / 5' % (d + 1))
    out = Image.new('RGBA', (96 * 6, 96 * 7))

    stand = img.crop(box(d, 0, 1))
    sit = img.crop(box(d + 5, 0, 1))
    out.paste(stand, (0, 0))
    out.paste(sit, (96, 0))

    for m in range(6):
        row = img.crop(box(d * 6, m + 1, 6))
        out.paste(row, (0, (m + 1) * 96))

    base, ext = os.path.splitext(path)
    out.save(base + ('-%d' % d) + ext)
