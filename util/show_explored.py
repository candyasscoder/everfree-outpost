import os
import sys

from PIL import Image

def main(dirname, out_file):
    player_pos = [] # TODO - load from a file or something
    player_pos = set((x // (16 * 32), y // (16 * 32)) for x,y,z in player_pos)

    explored = set()
    for filename in os.listdir(dirname):
        base, _ = os.path.splitext(filename)
        x, y = map(int, base.split(','))
        explored.add((x, y))

    min_x = min(x for x,y in explored)
    max_x = max(x for x,y in explored)
    min_y = min(y for x,y in explored)
    max_y = max(y for x,y in explored)

    width = max_x - min_x + 1
    height = max_y - min_y + 1

    img = Image.new('RGBA', (width, height))

    for x,y in explored:
        if (x,y) in player_pos:
            color = (0, 0, 255, 255)
        else:
            color = (0, 255, 0, 255)
        img.putpixel((x - min_x, y - min_y), color)

    img.putpixel((0 - min_x, 0 - min_y), (255, 0, 0, 255))

    img.resize(width * 3, height * 3)

    img.save(out_file)

if __name__ == '__main__':
    dirname, out_file = sys.argv[1:]
    main(dirname, out_file)

