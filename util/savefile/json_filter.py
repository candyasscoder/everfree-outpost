import json
import sys

if __name__ == '__main__':
    expr, = sys.argv[1:]
    x = json.load(sys.stdin)
    y = eval(expr, {'j': x})
    json.dump(y, sys.stdout)
