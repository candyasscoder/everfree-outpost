from collections import defaultdict
import math
import struct
import sys

from PIL import Image


def main(filename):
    with open(filename, 'rb') as f:
        buf = f.read()

    sz = round(math.sqrt(len(buf) / 4))

    hist = defaultdict(lambda: 0)

    img = Image.new('RGBA', (sz, sz), (0, 0, 0, 0))
    for i in range(sz * sz):
        val, = struct.unpack_from('i', buf, i * 4)
        x = i % sz
        y = i // sz
        if x == 0:
            print(y)

        hist[val] += 1

        scaled = (val + 64) * 2
        #img.putpixel((x, y), (scaled, scaled, scaled, 255))

    #img.save('ds.png')

    a = min(k for k in hist.keys())
    b = max(k for k in hist.keys())
    for i in range(a, b + 1):
        print('%4d: %7d (%4.1f%%)' % (i, hist[i], 100 * float(hist[i]) / (sz * sz)))

if __name__ == '__main__':
    filename, = sys.argv[1:]
    main(filename)
