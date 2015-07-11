import os

from configure.template import template
from configure.util import join, maybe


def rules(i):
    return template('''
        rule process_font
            command = $python3 $src/gen/process_font.py $
                --font-image-in=$in $
                --first-char=$first_char $
                --font-image-out=$out_img $
                --font-metrics-out=$out_metrics
            description = GEN $out_img

        rule process_day_night
            command = $python3 $src/gen/gen_day_night.py $in >$out
            description = GEN $out

        rule gen_server_json
            command = $python3 $src/gen/gen_server_json.py >$out
            description = GEN $out

        rule gen_credits
            command = cat $dep_files | $python3 $src/gen/gen_credits.py $src >$out
            description = GEN $out
    ''', **locals())

def font(out_base, src_img):
    out_img = out_base + '.png'
    out_metrics = out_base + '_metrics.json'

    return template('''
        build %out_img %out_metrics: process_font %src_img $
            | $src/gen/process_font.py
            first_char = 0x21
            out_img = %out_img
            out_metrics = %out_metrics
    ''', **locals())

def server_json(out_json):
    return template('''
        build %out_json: gen_server_json | $src/gen/gen_server_json.py
    ''', **locals())

def day_night(out_json, src_img):
    return template('''
        build %out_json: process_day_night %src_img $
            | $src/gen/gen_day_night.py
    ''', **locals())

def process():
    return template('''
        rule process_data
            command = rm -f $b_data/structures*.png && $
                $python3 $src/gen/data_main.py --mods=$mods $
                    --src-dir=$src --output-dir=$b_data && $
                touch $b_data/stamp
            description = DATA
            depfile = $b_data/data.d

        build $b_data/stamp $
            %for side in ('server', 'client')
                %for file in ('structures', 'blocks', 'items', 'recipes')
                    $b_data/%{file}_%{side}.json $
                %end
            %end
            $b_data/tiles.png $b_data/items.png: $
            process_data | $src/gen/data_main.py
    ''', **locals())

def pack():
    return template('''
        rule build_pack
            command = $python3 $src/mk/misc/make_pack.py $src $b_data $b_data/outpost.pack
            description = PACK
            depfile = $b_data/outpost.pack.d

        build $b_data/outpost.pack: build_pack $
            | $src/mk/misc/make_pack.py $
            || $b_data/stamp $b_data/font.png $b_data/day_night.json
    ''', **locals())

def credits(out_path):
    return template('''
        build %out_path: gen_credits $
            | $b_data/stamp $b_data/outpost.pack $
              $src/gen/gen_credits.py
            dep_files = $b_data/data.d $b_data/outpost.pack.d
    ''', **locals())
