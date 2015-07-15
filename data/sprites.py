from ..core.builder import *
from ..core.images import loader
from ..core.animation import AnimGroupDef
from ..core import util

#from ..core import sprite as S

from .lib import pony_sprites


def init():
    sheets = pony_sprites.get_base_sheets()

    for k,vs in sheets.items():
        for i,v in enumerate(vs):
            v.save('test-%s-%d.png' % (k, i))

    return

    base = loader('sprites/base')

    g = AnimGroupDef('pony')
    for i in range(5):
        for stance in ('stand', 'walk', 'run'):
            g.add_anim('%s-%d' % (stance, i),
                    6 if stance != 'stand' else 1,
                    10)
    g.finish()

    anims = list(g.anims.values())
    util.assign_ids(anims)
    from pprint import pprint
    pprint(S.build_client_json(anims))
    pprint(S.build_server_json(anims))





    layout = SheetLayout('pony',
            dict(('%s-%d' % (n,i), l)
                for i in range(5)
                for (n,l) in (('stand', 1), ('walk', 6), ('run', 6))))
    from pprint import pprint
    pprint(sorted((x.__dict__ for x in layout.anims.values()), key=lambda x:
        x['name']) )
