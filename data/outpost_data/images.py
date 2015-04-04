from PIL import Image

IMAGE_CACHE = {}

def load(path):
    if path not in IMAGE_CACHE:
        IMAGE_CACHE[path] = Image.open(path)
    return IMAGE_CACHE[path]

def get_loaded_paths():
    return IMAGE_CACHE.keys()
