from PIL import Image

from ..core.builder import *
from ..core.consts import *

def init():
    # Various code relies on these objects having well-known IDs.  If you need
    # to add to this list, you must also update the sets of "reserved names"
    # passed to `util.assign_ids` in gen/data/gen.py:postprocess.

    # Empty block (id = 0)
    mk_block('empty', 'empty', {})
    # `placeholder` (id = 1) is used to fill chunks that are waiting for real
    # block data to be generated.  It's solid to prevent players from moving or
    # placing structures in such chunks.  (In particular, it stops unusually
    # quick pegasi from bypassing not-yet-generated puzzles in dungeons.)
    mk_block('placeholder', 'solid', {})

    # "No item" (id = 0)
    mk_item('none', 'Nothing', Image.new('RGBA', (TILE_SIZE, TILE_SIZE)))
