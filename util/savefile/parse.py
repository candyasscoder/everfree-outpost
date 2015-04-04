import argparse
from collections import defaultdict
import json
import math
import os
import struct
import sys

from pprint import pprint


def build_parser():
    parser = argparse.ArgumentParser(
            description='Parse a save file to extract useful information')

    parser.add_argument('save_dir', metavar='SAVE_DIR',
            help='path to the save directory')

    parser.add_argument('--data-dir', metavar='DIR',
            help='path to the server data directory')

    parser.add_argument('--find-player-structures', action='store_true',
            help='find chunks containing player-built structures and write them to FILE')

    parser.add_argument('--dump-blocks', action='store_true',
            help='dump blocks arrays for chunks listed on stdin')

    return parser

def log(s):
    sys.stderr.write('%s\n' % s)


class Reader(object):
    def __init__(self, buf):
        self.buf = buf
        self.offset = 0
        self.template_map = {}
        self.item_map = {}

    def unpack_raw(self, fmt):
        result = struct.unpack_from(fmt, self.buf, self.offset)
        self.offset += struct.calcsize(fmt)
        if len(result) == 1:
            return result[0]
        else:
            return result

    def unpack(self, fmt):
        result = self.unpack_raw(fmt)
        # Round up to multiple of 4
        self.offset = (self.offset + 3) & ~3
        return result

    def u8(self):
        return self.unpack('B')

    def u16(self):
        return self.unpack('H')

    def u32(self):
        return self.unpack('I')

    def u64(self):
        return self.unpack('Q')

    def i8(self):
        return self.unpack('b')

    def i16(self):
        return self.unpack('h')

    def i32(self):
        return self.unpack('i')

    def i64(self):
        return self.unpack('q')

    def v3(self):
        return self.unpack('iii')

    def count(self):
        return self.u32()

    def str(self):
        count = self.count()
        return self.str_bytes(count)

    def str_bytes(self, count):
        raw = self.buf[self.offset : self.offset + count]
        result = raw.decode('utf-8')
        self.offset += count
        self.offset = (self.offset + 3) & ~3
        return result

    def template_id(self):
        id = self.u32()
        if id in self.template_map:
            return self.template_map[id]

        (x, y, z, name_len) = self.unpack('BBBB')
        size = (x, y, z)
        name = self.str_bytes(name_len)
        self.template_map[id] = name
        return self.template_map[id]

    def item_name_and_count(self):
        id, count, name_len = self.unpack('HBB')
        if id in self.item_map:
            return (self.item_map[id], count)

        name = self.str_bytes(name_len)
        self.item_map[id] = name
        return (self.item_map[id], count)

def parse_client(r):
    save_id = r.u32()
    r.u32()
    stable_id = r.u64()

    pawn_id = r.u32()
    name = r.str()

    extra = parse_extra(r)

    child_entities = []
    for i in range(r.count()):
        child_entities.append(parse_entity(r))

    child_inventories = []
    for i in range(r.count()):
        child_inventories.append(parse_inventory(r))

    return {
            'save_id': save_id,
            'stable_id': stable_id,
            'pawn_id': pawn_id,
            'name': name,
            'extra': extra,
            'child_entities': child_entities,
            'child_inventories': child_inventories,
            }

def parse_chunk(r):
    (save_id, _, cx, cy) = r.unpack('Iiii')

    blocks = []
    for i in range(16 * 16 * 16):
        blocks.append(r.unpack_raw('H'))

    block_map = {}
    for i in range(r.count()):
        (id, shape, name_len) = r.unpack('HBB')
        name = r.str_bytes(name_len)
        block_map[id] = (name, shape)

    child_structures = []
    for i in range(r.count()):
        child_structures.append(parse_structure(r))

    return {
            'save_id': save_id,
            'chunk_pos': (cx, cy),
            'blocks': blocks,
            'block_map': block_map,
            'child_structures': child_structures
            }

def parse_entity(r):
    save_id = r.u32()
    stable_id = r.u64()

    start_pos = r.v3()
    end_pos = r.v3()
    start_time = r.u64()
    (duration, anim) = r.unpack('HH')
    facing = r.v3()
    target_velocity = r.v3()
    appearance = r.u32()

    extra = parse_extra(r)

    child_inventories = []
    for i in range(r.count()):
        child_inventories.append(parse_inventory(r))

    return {
            'save_id': save_id,
            'stable_id': stable_id,

            'start_pos': start_pos,
            'end_pos': end_pos,
            'start_time': start_time,
            'duration': duration,
            'anim': anim,
            'facing': facing,
            'target_velocity': target_velocity,
            'appearance': appearance,

            'extra': extra,
            'child_inventories': child_inventories,
            }

def parse_structure(r):
    save_id = r.u32()
    stable_id = r.u64()

    pos = r.v3()
    template = r.template_id()

    extra = parse_extra(r)

    child_inventories = []
    for i in range(r.count()):
        child_inventories.append(parse_inventory(r))

    return {
            'save_id': save_id,
            'stable_id': stable_id,
            'pos': pos,
            'template': template,
            'extra': extra,
            'child_inventories': child_inventories,
            }

def parse_inventory(r):
    save_id = r.u32()
    stable_id = r.u64()

    contents = {}
    for i in range(r.count()):
        (name, count) = r.item_name_and_count()
        contents[name] = count

    extra = parse_extra(r)

    return {
            'save_id': save_id,
            'stable_id': stable_id,
            'contents': contents,
            'extra': extra
            }

T_NIL = 0
T_BOOL = 1
T_SMALL_INT = 2
T_LARGE_INT = 3
T_FLOAT = 4
T_SMALL_STRING = 5
T_LARGE_STRING = 6
T_TABLE = 7
T_WORLD = 8
T_CLIENT = 9
T_ENTITY = 10
T_STRUCTURE = 11
T_INVENTORY = 12
T_STABLE_CLIENT = 13
T_STABLE_ENTITY = 14
T_STABLE_STRUCTURE = 15
T_STABLE_INVENTORY = 16
T_V3 = 17

def parse_extra(r):
    (tag, a, b) = r.unpack('BBH')

    if tag == T_NIL:
        return None
    elif tag == T_BOOL:
        return a != 0
    elif tag == T_SMALL_INT:
        if b >= 0x8000:
            b -= 0x10000
        return b
    elif tag == T_LARGE_INT:
        return r.i32()
    elif tag == T_FLOAT:
        return r.unpack('d')
    elif tag == T_SMALL_STRING:
        return r.str_bytes(b)
    elif tag == T_LARGE_STRING:
        return r.str()
    elif tag == T_TABLE:
        result = {}
        while True:
            key = parse_extra(r)
            if key is None:
                break
            value = parse_extra(r)
            result[key] = value
        return result
    elif tag == T_WORLD:
        return ('world', 0)
    elif tag == T_CLIENT:
        return ('client', r.u16())
    elif tag == T_ENTITY:
        return ('entity', r.u32())
    elif tag == T_STRUCTURE:
        return ('structure', r.u32())
    elif tag == T_INVENTORY:
        return ('inventory', r.u32())
    elif tag == T_STABLE_CLIENT:
        return ('stable_client', r.u64())
    elif tag == T_STABLE_ENTITY:
        return ('stable_entity', r.u64())
    elif tag == T_STABLE_STRUCTURE:
        return ('stable_structure', r.u64())
    elif tag == T_STABLE_INVENTORY:
        return ('stable_inventory', r.u64())
    elif tag == T_V3:
        return (r.i32(), r.i32(), r.i32())
    assert False, 'unrecognized tag: %d' % tag

NATURAL_STRUCTURES = ('tree', 'stump', 'rock', 'chest')

def find_player_structures(save_dir):
    chunk_dir = os.path.join(save_dir, 'terrain_chunks')
    chunk_files = list(os.listdir(chunk_dir))

    log('%d chunk files to parse' % len(chunk_files))

    result = []

    for i, chunk_file in enumerate(chunk_files):
        if i % 100 == 0:
            log('%6d / %6d...' % (i, len(chunk_files)))

        path = os.path.join(chunk_dir, chunk_file)
        with open(path, 'rb') as f:
            buf = f.read()
        chunk = parse_chunk(Reader(buf))

        for s in chunk['child_structures']:
            if s['template'] not in NATURAL_STRUCTURES:
                base, _, _ = chunk_file.partition('.')
                x_str, _, y_str = base.partition(',')
                result.append((int(x_str), int(y_str)))
                break

    json.dump(result, sys.stdout)

def points(max_x, max_y, max_z):
    for z in range(max_z):
        for y in range(max_y):
            for x in range(max_x):
                yield (x, y, z)

def dump_blocks(save_dir, data_dir):
    with open(os.path.join(data_dir, 'blocks.json')) as f:
        blocks = json.load(f)
    with open(os.path.join(data_dir, 'objects.json')) as f:
        templates = json.load(f)

    block_map = dict((x['name'], i) for i, x in enumerate(blocks['blocks']))
    template_map = {}
    for obj in templates['objects']:
        size_x = obj['size_x']
        size_y = obj['size_y']
        size_z = obj['size_z']
        blocks = list(block_map[n] for n in obj['blocks'])
        template_map[obj['name']] = ((size_x, size_y, size_z), blocks)

    chunk_cache = {}
    def get_chunk(x, y):
        if (x,y) not in chunk_cache:
            path = os.path.join(save_dir, 'terrain_chunks', '%d,%d.terrain_chunk' % (x, y))
            with open(path, 'rb') as f:
                buf = f.read()
            chunk_cache[(x,y)] = parse_chunk(Reader(buf))
        return chunk_cache[(x, y)]

    result_map = {}
    chunks = json.load(sys.stdin)
    for i, (x, y) in enumerate(chunks):
        if i % 100 == 0:
            log('%5d / %5d' % (i, len(chunks)))

        chunk = get_chunk(x, y)
        chunk_block_map = dict((k, block_map[v[0]]) for k,v in chunk['block_map'].items())
        blocks = list(chunk_block_map[b] for b in chunk['blocks'])

        for dx, dy in [(0,0), (-1,0), (0,-1), (-1,-1)]:
            c = get_chunk(x + dx, y + dy)

            for s in c['child_structures']:
                size, s_blocks = template_map[s['template']]
                sx, sy, sz = s['pos']

                base_x = sx - x * 16
                base_y = sy - y * 16
                base_z = sz

                for i, (off_x, off_y, off_z) in enumerate(points(*size)):
                    bx = base_x + off_x
                    by = base_y + off_y
                    bz = base_z + off_z
                    if 0 <= bx < 16 and 0 <= by < 16 and 0 <= bz < 16:
                        idx = bx + 16 * (by + 16 * (bz))
                        blocks[idx] = s_blocks[i]

        result_map['%d,%d' % (x,y)] = blocks

    json.dump(result_map, sys.stdout)


def main():
    parser = build_parser()
    args = parser.parse_args()

    if args.find_player_structures:
        find_player_structures(args.save_dir)
    elif args.dump_blocks:
        dump_blocks(args.save_dir, args.data_dir)

def main_(save_dir):
    structure_counts = defaultdict(lambda: 0)
    item_counts = defaultdict(lambda: 0)

    chunk_dir = os.path.join(save_dir, 'terrain_chunks')
    for chunk_file in []:#os.listdir(chunk_dir):
        path = os.path.join(chunk_dir, chunk_file)
        print(path)
        with open(path, 'rb') as f:
            buf = f.read()
        chunk = parse_chunk(Reader(buf))

        for s in chunk['child_structures']:
            structure_counts[s['template']] += 1

            for i in s['child_inventories']:
                for k,v in i['contents'].items():
                    item_counts[k] += v

    clients_by_dist = []
    tribe_counts = defaultdict(lambda: 0)

    client_dir = os.path.join(save_dir, 'clients')
    for client_file in os.listdir(client_dir):
        path = os.path.join(client_dir, client_file)
        print(path)
        with open(path, 'rb') as f:
            buf = f.read()
        client = parse_client(Reader(buf))

        tribe_counts[client['name'][4]] += 1

        (x, y, z) = client['child_entities'][0]['start_pos']
        dist = math.sqrt(x*x + y*y + z*z)
        clients_by_dist.append((dist, client['name'], (x, y, z)))

        for e in client['child_entities']:
            for i in e['child_inventories']:
                for k,v in i['contents'].items():
                    item_counts[k] += v

        for i in client['child_inventories']:
            for k,v in i['contents'].items():
                item_counts[k] += v

    pprint(sorted(clients_by_dist)[-5:])
    pprint(structure_counts)
    pprint(item_counts)
    pprint(tribe_counts)

if __name__ == '__main__':
    main()

