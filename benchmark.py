#!/usr/bin/env python

import sys
import json
import uuid
import random
import socket
import time

def main():
    targets = sys.argv[1:]

    if not targets:
        print("Give me some hosts please")
        return 1

    targets = [ (ip, int(port)) for (ip, port) in [
        target.split(":", 1) for target in targets
    ] ]

    keys = [str(uuid.uuid4()) for i in range(256)]

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.SOL_UDP)
    sock.bind(("0.0.0.0", 55555))

    while True:
        sock.sendto(json.dumps({
            "Set": [random.choice(keys), str(uuid.uuid4())]
        }), random.choice(targets))
        sock.recvfrom(1024)
        time.sleep(.01)



if __name__ == '__main__':
    main()
