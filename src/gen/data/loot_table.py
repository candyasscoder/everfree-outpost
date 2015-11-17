from collections import namedtuple

from outpost_data.core import util


class ObjectRef(object):
    __slots__ = ('name', 'min_count', 'max_count', 'id')
    def __init__(self, name, min_count, max_count):
        self.name = name
        self.min_count = min_count
        self.max_count = max_count

        self.id = None

    def clone(self):
        return ObjectRef(self.name, self.min_count, self.max_count)

class TableRef(object):
    __slots__ = ('name', 'id')
    def __init__(self, name):
        self.name = name

        self.id = None

    def clone(self):
        return TableRef(self.name)

Weighted = namedtuple('Weighted', ('ref', 'weight'))
Weighted.clone = lambda self: Weighted(*self)

Optional = namedtuple('Optional', ('ref', 'chance'))
Optional.clone = lambda self: Optional(*self)


class ChooseBase(object):
    __slots__ = ('variants',)
    def __init__(self, variants=()):
        self.variants = []
        self.add_variants(variants)

    def clone(self):
        type(self)([v.clone() for v in self.variants])

    def add_variant(self, v):
        if isinstance(v, (ObjectRef, TableRef)):
            v = Weighted(v, 1)

        assert isinstance(v, Weighted), \
                'cannot add %s variant to %s table' % (type(v).__name__, type(self).__name__)
        assert isinstance(v.ref, (ObjectRef, TableRef)), \
                'cannot add %s variant to % table' % (type(v.ref).__name__, type(self).__name__)
        self.variants.append(v)

    def add_variants(self, vs):
        for v in vs:
            self.add_variant(v)

    def resolve_object_ids(self, id_map, name):
        for v in self.variants:
            if isinstance(v.ref, ObjectRef):
                v.ref.id = id_map.get(v.ref.name)
                if v.ref.id is None:
                    util.err('loot table %r: no such %s: %r' % (name, self.OBJ_KIND, v.ref.name))

class MultiBase(object):
    __slots__ = ('parts',)
    def __init__(self, parts=()):
        self.parts = []
        self.add_parts(parts)

    def clone(self):
        type(self)([p.clone() for p in self.parts])

    def add_part(self, p):
        if isinstance(p, (ObjectRef, TableRef)):
            p = Optional(p, 100)

        assert isinstance(p, Optional), \
                'cannot add %s part to %s table' % (type(p).__name__, type(self).__name__)
        assert isinstance(p.ref, (ObjectRef, TableRef)), \
                'cannot add %s part to %s table' % (type(p.ref).__name__, type(self).__name__)
        self.parts.append(p)

    def add_parts(self, ps):
        for p in ps:
            self.add_part(p)

    def resolve_object_ids(self, id_map, name):
        for p in self.parts:
            if isinstance(p.ref, ObjectRef):
                p.ref.id = id_map.get(p.ref.name)
                if p.ref.id is None:
                    util.err('loot table %r: no such %s: %r' % (name, self.OBJ_KIND, p.ref.name))


class ChooseItem(ChooseBase):
    OBJ_KIND = 'item'

class MultiItem(MultiBase):
    OBJ_KIND = 'item'

class ChooseStructure(ChooseBase):
    OBJ_KIND = 'structure'


class LootTableDef(object):
    def __init__(self, name, table, ext=False):
        self.name = name
        self.table = table
        self.ext = ext

    def resolve_object_ids(self, id_map):
        self.table.resolve_object_ids(id_map, self.name)

    def object_kind(self):
        return self.table.OBJ_KIND


def resolve_object_ids(tables, id_maps):
    for t in tables:
        if t.object_kind() == 'item':
            t.resolve_object_ids(id_maps.items)
        elif t.object_kind() == 'structure':
            t.resolve_object_ids(id_maps.structures)

def build_map(tables, kind):
    table_map = {}
    for t in (t for t in tables if t.object_kind() == kind and not t.ext):
        if t.name in table_map:
            util.err('multiple definitions of table %r' % t.name)
            continue
        table_map[t.name] = [t.table]

    for t in (t for t in tables if t.object_kind() == kind and t.ext):
        if t.name not in table_map:
            util.err('found extension of nonexistent table %r' % t.name)
            continue

        ts = table_map[t.name]
        if type(t.table) is not type(ts[0]):
            util.err('extension of table %r does not match original type (%s != %s)' %
                    (t.name, type(ts[0]).__name__, type(t.table)))
            continue

        ts.append(t.table)

    return table_map


"""
tables = [
    {
        'type': 'choose',
        'variants': [
            { 'type': 'object', 'id': 17, 'min_count': 1, 'max_count': 3, 'weight': 1 },
            { 'type': 'object', 'id': 18, 'min_count': 1, 'max_count': 1, 'weight': 2 },
            { 'type': 'table', 'id': 1, 'weight': 1 },
        ]
    },
"""

class Compiler(object):
    def __init__(self, tables, kind):
        """`tables` should already have been run through `resolve_object_ids`"""
        self.table_map = build_map(tables, kind)
        self.compiled = []
        self.id_map = {}
        self.object_id_map = {}

        self.use_count = kind == 'item'

        # Pre-assign IDs to tables.
        for k in sorted(self.table_map.keys()):
            self.id_map[k] = len(self.compiled)
            self.compiled.append(None)

    def resolve_object_ref(self, src_name, ref):
        key = (ref.id, ref.min_count, ref.max_count)
        if key not in self.object_id_map:
            self.object_id_map[key] = len(self.compiled)

            # Generate the compiled object table.
            dct = { 'type': 'object', 'id': ref.id }
            if self.use_count:
                dct['min_count'] = ref.min_count
                dct['max_count'] = ref.max_count
            self.compiled.append(dct)
        return self.object_id_map[key]

    def resolve_table_ref(self, src_name, ref):
        if ref.name not in self.id_map:
            util.err('table %r refers to nonexistent table %r' %
                    (src_name, ref.name))
            return None
        return self.id_map[ref.name]

    def resolve_ref(self, src_name, ref):
        if isinstance(ref, ObjectRef):
            return self.resolve_object_ref(src_name, ref)
        elif isinstance(ref, TableRef):
            return self.resolve_table_ref(src_name, ref)
        else:
            assert False, 'unexpected ref type: %r' % type(ref)

    def merge_weights(self, tables):
        weight_map = {}
        ref_map = {}
        for t in tables:
            for v in t.variants:
                if isinstance(v.ref, TableRef):
                    key = (TableRef, v.ref.name)
                elif isinstance(v.ref, ObjectRef):
                    key = (ObjectRef, v.ref.name, v.ref.min_count, v.ref.max_count)
                else:
                    assert False, 'unexpected ref type: %r' % type(v.ref)

                if key not in ref_map:
                    ref_map[key] = v.ref
                    weight_map[key] = 0
                weight_map[key] += v.weight

        result = []
        for key, ref in ref_map.items():
            w = weight_map[key]
            if w <= 0:
                continue
            result.append(Weighted(ref, w))

        return result

    def compile_table(self, name, tables):
        if isinstance(tables[0], ChooseBase):
            c = self.compile_table_choose(name, tables)
        elif isinstance(tables[0], MultiBase):
            c = self.compile_table_multi(name, tables)
        else:
            assert False, 'unrecognized table type: %s' % type(tables[0])
        self.compiled[self.id_map[name]] = c

    def compile_table_choose(self, name, tables):
        variants = self.merge_weights(tables)
        variant_dcts = []
        for v in variants:
            table_id = self.resolve_ref(name, v.ref)
            variant_dcts.append({'id': table_id, 'weight': v.weight})
        return {
                'type': 'choose',
                'name': name,
                'variants': variant_dcts,
                }

    def compile_table_multi(self, name, tables):
        part_dcts = []
        for t in tables:
            for p in t.parts:
                table_id = self.resolve_ref(name, p.ref)
                part_dcts.append({'id': table_id, 'chance': p.chance})
        return {
                'type': 'multi',
                'name': name,
                'parts': part_dcts,
                }

    def cycle_check(self):
        stack = []
        on_stack = set()
        visited = set()

        def go(i):
            if i in on_stack:
                idx = stack.index(i)
                names = [self.compiled[j]['name'] for j in stack[idx:]]
                names.append(names[0])
                raise ValueError('detected cycle among loot tables: %s' % names)

            if i in visited:
                return

            stack.append(i)
            on_stack.add(i)
            visited.add(i)

            c = self.compiled[i]
            if c['type'] == 'choose':
                for v in c['variants']:
                    go(v['id'])
            elif c['type'] == 'multi':
                for v in c['parts']:
                    go(v['id'])
            elif c['type'] == 'object':
                pass
            else:
                assert False, 'unrecognized compiled table type: %r' % (c['type'],)

            on_stack.remove(i)
            stack.pop()

        for i in range(len(self.compiled)):
            go(i)

    def compile(self):
        for name, tables in self.table_map.items():
            self.compile_table(name, tables)
        #self.cycle_check()
        return self.compiled


def build_server_json(tables):
    return {
            'items': Compiler(tables, 'item').compile(),
            'structures': Compiler(tables, 'structure').compile(),
            }
