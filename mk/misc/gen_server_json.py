import json
import sys
import time

if __name__ == '__main__':
    obj = {
            'url': 'ws://localhost:8888/ws',
            'world_version': int(time.time()),
            }
    json.dump(obj, sys.stdout)
