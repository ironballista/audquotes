#!/usr/bin/env python3

import schedule
import time
import random
import os

from glob import iglob
from atproto import Client

quotes = tuple()
indices = []
random.seed(time.monotonic())

def auth_client():
    handle, password, = os.getenv('BLUESKY_USERNAME'), os.getenv('BLUESKY_PASSWORD')
    client = Client()
    profile = client.login(handle, password)
    return client

def sample_quote(client):
    global quotes, indices
    path = quotes[indices.pop()]

    with open(path, 'r') as f:
        post_text = f.read().strip()

    client.send_post(post_text)

def main():
    global quotes, indices

    client = auth_client()
    quotes = tuple(iglob('src/**/*.txt', recursive=True))
    indices = []

    schedule.every().hour.at(':00').do(sample_quote, client)
    schedule.every().hour.at(':30').do(sample_quote, client)
    # schedule.every(10).seconds.do(sample_quote, client)

    while True:
        if len(indices) == 0:
            indices = random.sample(range(len(quotes)), k=len(quotes))
            # [print(quotes[i]) for i in indices]

        schedule.run_pending()
        time.sleep(1)

if __name__ == '__main__':
    main()
