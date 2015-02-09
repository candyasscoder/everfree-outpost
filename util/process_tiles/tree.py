from collections import namedtuple

Node = namedtuple('Node', ['children'])
Leaf = namedtuple('Leaf', ['value'])

def expand_tree(yaml, delims='/'):
    def put(node, path, value):
        for k in path[:-1]:
            if k not in node:
                node[k] = {}
            node = node[k]

        k = path[-1]
        if k in node and isinstance(value, dict):
            old = node[k]
            assert isinstance(old, dict), \
                    'tried to merge dict with nondict at path %r' % (path,)
            for j in value:
                assert j not in old, \
                        'tried to overwrite key %r during merge at path %r' % (j, path)
                old[j] = value[j]
        else:
            assert k not in node, \
                    'tried to overwrite existing value at path %r' % (path,)
            node[k] = value

    def go(yaml):
        if not isinstance(yaml, dict):
            return yaml if yaml is not None else {}

        node = {}
        for k,v in yaml.items():
            path = [k]
            for delim in delims:
                path = [x for xs in path for x in xs.split(delim)]
            put(node, path, go(v))
        return node

    return go(yaml)

def tag_leaf_dicts(tree, subdicts={}):
    def go(tree):
        if all(k in subdicts or not isinstance(v, dict) for k,v in tree.items()):
            return Leaf(tree)
        else:
            for k,v in tree.items():
                tree[k] = go(v)
            return tree
    go(tree)

def tag_leaf_values(tree):
    def go(tree):
        if not isinstance(tree, dict):
            return Leaf(tree)
        else:
            for k,v in tree.items():
                tree[k] = go(v)
            return tree
    go(tree)

def apply_defaults(tree, default_key='_', combine={}):
    def default_combine(new, old):
        return new

    def go(tree, defaults):
        if isinstance(tree, Leaf):
            for k,v in defaults.items():
                if k not in tree.value:
                    tree.value[k] = v
                elif k in combine:
                    tree.value[k] = combine[k](tree.value[k], v)
        else:
            default_leaf = tree.pop(default_key, None)
            if default_leaf is not None:
                new_defaults = default_leaf.value
                old_defaults = defaults
                defaults = {}

                for k in old_defaults.keys() | new_defaults.keys():
                    old = old_defaults.get(k)
                    new = new_defaults.get(k)
                    defaults[k] = combine.get(k, default_combine)(new, old)

            for v in tree.values():
                go(v, defaults)

    go(tree, {})

def flatten_tree(tree):
    result = {}

    def go(tree, base):
        if isinstance(tree, Leaf):
            result[base] = tree.value
        else:
            for k,v in tree.items():
                go(v, base + '/' + k)

    for k,v in tree.items():
        go(v, k)
    return result


if __name__ == '__main__':
    import sys
    from pprint import pprint
    import yaml
    y = yaml.load(sys.stdin)
    t = expand_tree(y, '/.')

    def combine_prefix(new, old):
        if old is None:
            return new
        elif new is None:
            return old
        else:
            return old + '/' + new

    tag_leaf_dicts(t)
    apply_defaults(t, combine={'prefix': combine_prefix})

    pprint(flatten_tree(t))
