from ..core.builder import *
from ..core.images import loader
from ..core.structure import Shape, Model, StaticAnimDef
from ..core.util import extract, chop_image

from .lib.items import *
from .lib.structures import *


def do_statue(basename, image):
    b = structure_builder()

    parts = [image.crop((x * 64, 0, (x + 1) * 64, 96)) for x in (0, 1, 2)]
    b.merge(mk_solid_structure(basename + '/n', parts[0], (2, 1, 2)))
    b.merge(mk_solid_structure(basename + '/s', parts[1], (2, 1, 2)))
    b.merge(mk_solid_structure(basename + '/e', parts[2], (2, 1, 2)))

    flipped = parts[2].transpose(Image.FLIP_LEFT_RIGHT)
    b.merge(mk_solid_structure(basename + '/w', flipped, (2, 1, 2)))

    return b

def init():
    tiles = loader('tiles')
    icons = loader('icons')
    structures = loader('structures')

    image = structures('furniture.png')

    bed_model = Model(
            models.quad_y(64,  8, 56,  0, 10) +
            models.quad_z(10,  8, 56,  0, 62) +
            models.quad_y( 2,  8, 56, 10, 20))
    s = structure_builder()
    s.merge(mk_solid_structure('bed', image, (2, 2, 1), (0, 0), model=bed_model))
    mk_structure_item(s['bed'], 'bed', 'Bed') \
            .recipe('anvil', {'wood': 20})

    s.merge(mk_solid_structure('table', image, (2, 2, 1), (2, 0)))
    mk_structure_item(s['table'], 'table', 'Table') \
            .recipe('anvil', {'wood': 20})

    cabinet_model = Model(
            models.quad_y(29,  0, 32,  0, 60) +
            models.quad_z(60,  0, 32, 20, 29))
    s.merge(mk_solid_structure('cabinets', image, (1, 1, 2), (4, 0),
            model=cabinet_model, layer=2))
    mk_structure_item(s['cabinets'], 'cabinets', 'Cabinets', (0, 1)) \
            .recipe('anvil', {'wood': 20})

    s.merge(mk_solid_structure('bookshelf/0', image, (1, 1, 2), (5, 0),
            model=cabinet_model, layer=2))
    mk_structure_item(s['bookshelf/0'], 'bookshelf', 'Bookshelves', (0, 1)) \
            .recipe('anvil', {'wood': 20})
    mk_solid_structure('bookshelf/1', image, (1, 1, 2), (6, 0),
            model=cabinet_model, layer=2)
    mk_solid_structure('bookshelf/2', image, (1, 1, 2), (7, 0),
            model=cabinet_model, layer=2)
    mk_item('book', 'Book', icons('gervais_roguelike/AngbandTk_book.png'))


    s.merge(mk_solid_structure('trophy', structures('trophy.png'), (1, 1, 1)))
    mk_structure_item(s['trophy'], 'trophy', 'Trophy') \

    s.merge(mk_solid_structure('fountain', structures('fountain.png'), (2, 2, 1)))
    mk_structure_item(s['fountain'], 'fountain', 'Fountain') \

    torch_frames = chop_image(structures('torch.png'), (TILE_SIZE, 2 * TILE_SIZE))
    torch_anim = StaticAnimDef(list(torch_frames.values()), 4)
    s.merge(mk_solid_structure('torch', torch_anim, (1, 1, 1)) \
            .light((16, 16, 32), (255, 230, 200), 300))
    mk_structure_item(s['torch'], 'torch', 'Torch') \
            .recipe('anvil', {'wood': 2, 'stone': 1})

    s.merge(do_statue('statue', structures('statue.png')))
    mk_structure_item(s['statue/e'], 'statue', 'Statue') \
            .recipe('anvil', {'stone': 50})

    stair_verts_tile = models.quad((0, 1, 0), (1, 1, 0), (1, 0, 1), (0, 0, 1))
    stair_model = Model([tuple(x * TILE_SIZE for x in vert) for vert in stair_verts_tile])

    stair_img = structures('stair.png')
    stair_shape = Shape(1, 1, 1, ['ramp_n'])
    stair = mk_structure('stair/n', stair_img, stair_model, stair_shape, 1)
    mk_structure_item(stair['stair/n'], 'stair', 'Stairs') \
            .recipe('anvil', {'wood': 10})
