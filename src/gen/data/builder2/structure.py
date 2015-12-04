from outpost_data.core import image2
from outpost_data.core.builder2.base import *
from outpost_data.core.consts import *
from outpost_data.core.structure import solid, StructureDef, StructureDef2, StaticAnimDef


DEFAULT_SHAPE = solid(1, 1, 1)

class StructurePrototype(PrototypeBase):
    KIND = 'structure'
    FIELDS = (
            'image', 'model', 'shape', 'layer', 'parts',
            'light_offset', 'light_color', 'light_radius',
            'anim_frames', 'anim_framerate', 'anim_oneshot',
            )

    def instantiate(self):
        self.name = self.require('name') or '_%x' % id(self)

        shape = self.require('shape', DEFAULT_SHAPE)
        layer = self.require('layer', 0)

        if self.require_one('model', 'parts'):
            if self.require_one('image', 'anim_frames'):
                img = raw_image(self.image)
            else:
                frames = self.anim_frames
                rate = self.require('anim_framerate', default=1, reason='anim_frames')
                oneshot = self.anim_oneshot or False

                img = StaticAnimDef(self.anim_frames, self.anim_framerate, self.anim_oneshot)
            model = self.model

            s = StructureDef(self.name, img, model, shape, layer)
        else:
            parts = self.parts
            self.require_unset('image', 'parts')
            self.require_unset('anim_frames', 'parts')
            self.require_unset('anim_framerate', 'parts')
            self.require_unset('anim_oneshot', 'parts')

            s = StructureDef2(self.name, shape, layer)
            for m, i in parts:
                s.add_part(m, i)

        pos, color, radius = self.check_group(
                ('light_offset', 'light_color', 'light_radius'))
        if pos is not None:
            s.set_light(pos or (0, 0, 0), color or (0, 0, 0), radius or 1)

        return s

    def get_image(self):
        if self.image is not None:
            return self.image
        elif self.anim_frames is not None and len(self.anim_frames) > 0:
            return self.anim_frames[0]
        else:
            return image2.Image(img=DEFAULT_IMAGE)

class StructureBuilder(BuilderBase):
    PROTO_CLASS = StructurePrototype

    image = dict_modifier('image')
    model = dict_modifier('model')
    shape = dict_modifier('shape')
    layer = dict_modifier('layer')
    parts = dict_modifier('parts')

    light_offset = dict_modifier('light_offset')
    light_color = dict_modifier('light_color')
    light_radius = dict_modifier('light_radius')

    anim_frames = dict_modifier('anim_frames')
    anim_framerate = dict_modifier('anim_framerate')
    anim_oneshot = dict_modifier('anim_oneshot')

    def light(offset, color, radius):
        def f(x):
            x.light_offset = offset
            x.light_color = color
            x.light_radius = radius
        return self._modify(f)

    def anim(frames, framerate, oneshot=False):
        def f(x):
            x.anim_frames = frames
            x.anim_framerate = framerate
            x.anim_oneshot = oneshot
        return self._modify(f)

    def part(self, *args):
        if len(args) == 1:
            args, = args

        def f(x, part):
            if x.parts is None:
                x.parts = [part]
            else:
                x.parts.append(part)
        self._dict_modify(f, args)
