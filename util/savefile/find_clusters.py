from collections import deque
import json
import sys

def log(s):
    sys.stderr.write('%s\n' % s)

def main():
    nodes = []
    for line in sys.stdin:
        x, y = map(int, line.strip().split())
        nodes.append((x, y))
    log('loaded %d points' % len(nodes))
    node_set = set(nodes)


    edges = dict()
    def add_edge(a, b):
        if a not in node_set or b not in node_set:
            return

        if a not in edges:
            edges[a] = []
        if b not in edges:
            edges[b] = []
        edges[a].append(b)
        edges[b].append(a)

    for x, y in nodes:
        if (x, y) not in edges:
            edges[(x, y)] = []

        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            add_edge((x, y), (x + dx, y + dy))
    log('built graph')


    component_idx = {}
    components = []

    for p in nodes:
        if p in component_idx:
            # Already assigned to a component
            continue

        comp = len(components)
        components.append([])

        def walk(n):
            if n in component_idx:
                return

            component_idx[n] = comp
            components[comp].append(n)

            for m in edges[n]:
                walk(m)

        walk(p)

    json.dump(components, sys.stdout)


if __name__ == '__main__':
    main()

