# already imported: gimp, pdb
import os

BLACKLIST = ('orig', 'slicing', 'guide', 'fill', 'sample', 'test')

def export_layers(n, l):
    for x in BLACKLIST:
        if x in l.name:
            return

    base, _, _ = l.name.partition('#')
    base = base.strip().replace(' ', '-')
    name = n + '-' + base
    if len(l.children) == 0:
        filename = '%s/%s.png' % (os.environ['GIMP_LAYER_EXPORT_DIR'], name)
        pdb.file_png_save(l.image, l, filename, filename,
                False, 3, False, False, False, False, False)
    else:
        for c in l.children:
            export_layers(name, c)

def export_image_layers(i):
    name, _ = os.path.splitext(os.path.basename(i.filename))
    for l in i.layers:
        export_layers(name, l)

def main():
    for img in gimp.image_list():
        export_image_layers(img)

try:
    main()
except:
    import traceback
    traceback.print_exc()

pdb.gimp_quit(False)
