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
            'sunrise': get_row(img, 0),
            'sunset': get_row(img, 1),
            }

    json.dump(j, sys.stdout)

if __name__ == '__main__':
    filename, = sys.argv[1:]
    main(filename)
