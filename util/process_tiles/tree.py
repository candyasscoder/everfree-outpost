import re


def flatten_tree(yaml, delims='/.'):
    delim_re = re.compile('[%s]' % delims)

    result = {}
    def go(k, v):
        if isinstance(v, dict):
            if len(v) == 0:
                result[k] = v
            else:
                for k2, v2 in v.items():
                    k2 = tuple(delim_re.split(k2))
                    go(k + k2, v2)
        else:
            assert k not in result, \
                    'duplicate definition for path %r' % (k,)
            result[k] = v

    go((), yaml)

    return result

def unflatten_tree(dct):
    def put(k, v, out):
        if len(k) == 1:
            out[k[0]] = v
        else:
            first = k[0]
            rest = k[1:]
            if first not in out:
                out[first] = {}
            put(rest, v, out[first])

    result = {}
    for k,v in dct.items():
        put(k, v, result)
    return result

def expand_tree(yaml, delims='/.'):
    return unflatten_tree(flatten_tree(yaml, delims=delims))

MISSING = {}

def default_merge(default, current):
    if current is MISSING:
        return default
    else:
        return current

def maybe_set(dct, k, v):
    if v is MISSING:
        if k in dct:
            del dct[k]
    else:
        dct[k] = v

def apply_defaults(tree, merge={}):
    def go(dct):
        defaults = dct.pop('_', {})

        for subdct in dct.values():
            if not isinstance(subdct, dict):
                continue

            for k, def_val in defaults.items():
                sub_val = subdct.get(k, MISSING)
                m = merge.get(k, default_merge)
                maybe_set(subdct, k, m(def_val, sub_val))

            go(subdct)

    go(tree)

def propagate(tree, combine={}):
    def go(dct):
        for subdct in dct.values():
            if not isinstance(subdct, dict):
                continue

            for k, c in combine.items():
                if k not in dct:
                    continue

                parent_val = dct[k]
                child_val = subdct.get(k, MISSING)
                maybe_set(subdct, k, c(parent_val, child_val))

            go(subdct)

    go(tree)

def collect_dicts(tree, fields=(), dict_fields=(), delim='/'):
    fields = set(fields)
    dict_fields = set(dict_fields)

    def is_item(dct):
        if not isinstance(dct, dict):
            return False

        # Have we seen at least one of the fields listed in 'fields'?
        saw_good_field = False
        # Have we seen a dict under a key not listed in 'dict_fields'?
        saw_bad_dict = False
        for k,v in dct.items():
            if isinstance(v, dict) and k not in dict_fields:
                saw_bad_dict = True
            if k in fields:
                saw_good_field = True
        return saw_good_field and not saw_bad_dict

    return collect(tree, is_item, delim=delim)

def collect_scalars(tree, delim='/'):
    def is_item(x):
        return not isinstance(x, dict)

    return collect(tree, is_item, delim=delim)


def collect(tree, is_item, delim='/'):
    result = {}

    def go(k, v):
        if is_item(v):
            result[k] = v
        else:
            if not isinstance(v, dict):
                return

            for k2, v2 in v.items():
                go('%s%s%s' % (k, delim, k2), v2)

    for k, v in tree.items():
        go(k, v)

    return result
