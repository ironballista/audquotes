#!/usr/bin/env python3

from atproto import Client, client_utils

import argparse
import logging
import datetime as dt
import os
import sys

parser = argparse.ArgumentParser(description="Interact with the Bluesky API via CLI.")
parser.add_argument('-f,--file', dest="post_filename", action="store",
        help="file from which to retrieve post contents", required=False)

args = parser.parse_args()

logging.basicConfig(filename=f'config/{dt.datetime.now():%Y-%m}-DEBUG.log',
        encoding='utf-8')

logger = logging.getLogger('audsky')
logger.setLevel(logging.DEBUG)

def get_time_fmt():
    return f"{dt.datetime.now():%Y-%m-%d @%H:%M:%S}"

def main():
    try:
        if args.post_filename is not None:
            with open(args.post_filename, 'r') as f:
                post_text = f.read().strip()
        else:
            post_text = sys.stdin.read().strip()
    except OSError as e:
        print("[ERROR]: Could not read from file.")
        logger.error(f"[{get_time_fmt()}] Could not read from file: {args.post_filename}; " +
                os.strerror(e.errno))
        sys.exit(1)

    handle, password, = os.getenv('BLUESKY_USERNAME'), os.getenv('BLUESKY_PASSWORD')

    client = Client()
    profile = client.login(handle, password)

    logger.debug(f"[{get_time_fmt()}] Posting as @{handle}; " +
            f"file location: {args.post_filename}")

    if len(post_text) <= 300:
        post = client.send_post(post_text)
        sys.exit(0)
    else:
        print("[ERROR]: Tweet text too long!")
        logger.error(f"[{get_time_fmt()}] Tweet contents too long; argument length: " +
                f"{len(post_text)}; file location: {args.post_filename}")
        sys.exit(1)


if __name__ == '__main__':
    main()
