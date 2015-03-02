import json
import sys

def main():
    nodes = set(map(tuple, json.load(sys.stdin)))
    result = set()
    for x,y in nodes:
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                result.add((x + dx, y + dy))
    json.dump(list(result), sys.stdout)

if __name__ == '__main__':
    main()
