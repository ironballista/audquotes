#!/usr/bin/env python3

import time
import random
import os
import logging
import hashlib

from glob import iglob
from typing import Optional

import atproto
import atproto.exceptions
import schedule
import typer

def auth_client() -> atproto.Client:
    handle, password, = os.getenv('BLUESKY_USERNAME'), os.getenv('BLUESKY_PASSWORD')

    client: atproto.Client = atproto.Client()

    try:
        _profile: atproto.models.AppBskyActorDefs.ProfileViewDetailed = client.login(handle, password)
    except atproto.exceptions.AtProtocolError as e:
        logging.error("Could not log into profile `%s`: %s", hashlib.sha256(handle.encode()).hexdigest(), e)
        raise

    logging.info("Logged into profile `%s` successfully.", hashlib.sha256(handle.encode()).hexdigest())
    return client

def sample_quote(client: Optional[atproto.Client], simulation_mode: bool = False) -> None:
    global quotes, indices
    path = quotes[indices.pop()]

    with open(path, 'r') as f:
        post_text: str = f.read().strip()

    if simulation_mode:
        print(post_text, '\n')
        return

    try: 
        client.send_post(post_text)
    except atproto.exceptions.AtProtocolError as e:
        logging.error("Error occurred when attempting to post `%s`: %s", path, e)
        raise

def main(simulation_mode: bool = True, log_level: str = "INFO") -> None:
    global quotes, indices

    logging.getLogger().setLevel(os.getenv("LOGGING") or log_level)

    quotes = tuple(iglob('src/**/*.txt', recursive=True))
    indices = []

    logging.debug("Initialized `quotes` tuple with the following values: %s", quotes)

    if simulation_mode:
        schedule.every(10).seconds.do(sample_quote, None, simulation_mode=True)
    else:
        client: atproto.Client = auth_client()
        schedule.every().hour.at(':00').do(sample_quote, client)
        schedule.every().hour.at(':30').do(sample_quote, client)

    while True:
        if len(indices) == 0:
            indices = random.sample(range(len(quotes)), k=len(quotes))

            if logging.getLogger().isEnabledFor(logging.DEBUG):
                shuffled_quotes = tuple(quotes[i] for i in indices)
                logging.debug("Shuffled `quotes` tuple as follows: %s", shuffled_quotes)

        schedule.run_pending()
        time.sleep(1)

if __name__ == '__main__':
    random.seed(time.monotonic())
    typer.run(main)
