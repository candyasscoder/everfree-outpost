from outpost_data.builder import *
import outpost_data.images as I
from outpost_data import depthmap
from outpost_data.structure import Shape
from outpost_data.util import loader, extract

from lib.items import *
from lib.structures import *


def do_statue(basename, image):
    b = structure_builder()

    parts = [image.crop((x * 64, 0, (x + 1) * 64, 96)) for x in (0, 1, 2)]
    b.merge(mk_solid_structure(basename + '/n', parts[0], (2, 1, 2)))
    b.merge(mk_solid_structure(basename + '/s', parts[1], (2, 1, 2)))
    b.merge(mk_solid_structure(basename + '/e', parts[2], (2, 1, 2)))

    flipped = parts[2].transpose(Image.FLIP_LEFT_RIGHT)
    b.merge(mk_solid_structure(basename + '/w', flipped, (2, 1, 2)))

    return b

def init(asset_path):
    tiles = loader(asset_path, 'tiles')
    structures = loader(asset_path, 'structures')

    image = structures('furniture.png')
    plane = structures('furniture-planemap.png')

    s = structure_builder()
    s.merge(mk_solid_structure('bed', image, (2, 2, 1), (0, 0), plane_image=plane))
    mk_structure_item(s['bed'], 'bed', 'Bed') \
            .recipe('anvil', {'wood': 20})

    s.merge(mk_solid_structure('table', image, (2, 2, 1), (2, 0), plane_image=plane))
    mk_structure_item(s['table'], 'table', 'Table') \
            .recipe('anvil', {'wood': 20})

    s.merge(mk_solid_structure('cabinets', image, (1, 1, 2), (4, 0),
            plane_image=plane, layer=2))
    mk_structure_item(s['cabinets'], 'cabinets', 'Cabinets', (0, 1)) \
            .recipe('anvil', {'wood': 20})

    s.merge(mk_solid_structure('bookshelf/0', image, (1, 1, 2), (5, 0),
            plane_image=plane, layer=2))
    mk_structure_item(s['bookshelf/0'], 'bookshelves', 'Bookshelves', (0, 1))
    mk_solid_structure('bookshelf/1', image, (1, 1, 2), (6, 0), plane_image=plane, layer=2)
    mk_solid_structure('bookshelf/2', image, (1, 1, 2), (7, 0), plane_image=plane, layer=2)
    mk_item('book', 'Book', tiles('gervais_roguelike/AngbandTk_book.png'))

    s.merge(mk_solid_structure('trophy', structures('trophy.png'), (1, 1, 1)))
    mk_structure_item(s['trophy'], 'trophy', 'Trophy')

    s.merge(mk_solid_structure('fountain', structures('fountain.png'), (2, 2, 1)))
    mk_structure_item(s['fountain'], 'fountain', 'Fountain')

    s.merge(mk_solid_structure('torch', structures('torch.png'), (1, 1, 1)) \
            .light((16, 16, 32), (255, 230, 200), 300))
    mk_structure_item(s['torch'], 'torch', 'Torch') \
            .recipe('anvil', {'wood': 2, 'stone': 1})

    s.merge(do_statue('statue', structures('statue.png')))
    mk_structure_item(s['statue/e'], 'statue', 'Statue') \
            .recipe('anvil', {'stone': 50})
