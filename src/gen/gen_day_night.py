import json
import sys

from PIL import Image


def get_row(img, row):
    result = []
    for r, g, b, a in img.crop((1, row, 9, row + 1)).getdata():
        result.append((r, g, b))
    return result

def main(filename):
    img = Image.open(filename)

    j = {
            'sunrise': get_row(img, 0) + [(255, 255, 255)],
            'sunset': get_row(img, 1) + [(255, 255, 255)],

            # Duration of a single day/night cycle is 24000 units.
            'day_start': 0,
            'day_end': 14000,
            'night_start': 16000,
            'night_end': 22000,
            }

    json.dump(j, sys.stdout)

if __name__ == '__main__':
    filename, = sys.argv[1:]
    main(filename)
