#!/usr/bin/env python3

import time
import random
import os
import logging
import hashlib
import copy

from glob import iglob
from typing import Optional
from pathlib import Path

import redis
import typer

def main(source_key, destination_key, rename_key: bool = False, password: str = "", host: str = "localhost", port: int = 16379, quotes_pattern: str = "quotes/**/*.txt", log_level: str = "INFO") -> None:
    logging.getLogger().setLevel(os.getenv("LOGGING") or log_level)

    new_quotes = tuple(iglob(quotes_pattern, recursive=True))
    r = redis.Redis(host=host, port=port, decode_responses=True, password=password)

    logging.info("Fetching existing list at key `%s`.", source_key)
    current_list = r.lrange(source_key, "0", "-1")
    logging.info("Found list of length %d.", len(current_list))

    new_list = copy.deepcopy(current_list)
    new_list.extend(Path(p).read_text().strip() for p in new_quotes)
    random.shuffle(new_list)

    if (actual_len := r.llen(destination_key)) > 0:
        logging.error("Attempted to push result to non-empty key `%s` (length: %d). Aborting.", destination_key, actual_len)
        return

    r.lpush(destination_key, *new_list)
    logging.info("Pushed new, shuffled list (length: %d) successfully to key `%s`.", len(new_list), destination_key)

    if rename_key:
        logging.info("Renaming `%s` to `%s`.", destination_key, source_key)
        r.rename(destination_key, source_key)


if __name__ == '__main__':
    random.seed(time.monotonic())
    typer.run(main)
