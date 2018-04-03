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
        writer = random.choice(targets)
        key    = random.choice(keys)
        value  = str(uuid.uuid4())
        sock.sendto(json.dumps({"Set": [key, value]}), writer)

        data, addr = sock.recvfrom(1024)
        response = json.loads(data)
        if response != "Ok":
            print(response)
        time.sleep(.003)

        for target in targets:
            if target == writer:
                continue

            sock.sendto(json.dumps({"Get": key}), target)

            data, addr = sock.recvfrom(1024)
            response = json.loads(data)
            if response["Value"][1] != value:
                print("[%-16s] Expected %s, got %r" % (writer[0], value, response["Value"][1]))
            else:
                print("[%-16s] Response is correct" % writer[0])
            time.sleep(.001)


if __name__ == '__main__':
    main()
