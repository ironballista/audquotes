#!/usr/bin/env python3

import tweepy
import argparse
import logging
import datetime as dt
import os
import sys

# Read in API keys from file
with open('./config/secret', "r") as infile:
    consumer_key, consumer_secret, *_ = infile.read().split("\n")

auth = tweepy.OAuthHandler(consumer_key, consumer_secret)

parser = argparse.ArgumentParser(description="Interact with the Twitter API via CLI.")
parser.add_argument("-c,--credentials", dest="credentials_filename", action="store",
        default="", help="filename to look for credentials")
subparsers = parser.add_subparsers()

parser_send = subparsers.add_parser('send', help='updates the user status')
# parser_send.add_argument('text', action="store", help="contents of tweet")
parser_send.add_argument('-f,--file', dest="tweet_filename", action="store",
        help="file from which to retrieve tweet contents", required=True)

args = parser.parse_args()

logging.basicConfig(filename=f'config/{dt.datetime.now():%Y-%m}-DEBUG.log',
        encoding='utf-8')

logger = logging.getLogger('audtweet')
logger.setLevel(logging.DEBUG)

def get_time_fmt():
    return f"{dt.datetime.now():%Y-%m-%d @%H:%M:%S}"

if not args.credentials_filename:
    website = auth.get_authorization_url()
    print(f"Please open {website} in your browser, log in, and",
           "retrieve the verifier token.")

    verifier = input("Verifier: ")
    auth.get_access_token(verifier)

    api = tweepy.API(auth)

    user = api.me()
    print(f"Logged in as @{user.screen_name}!")
    usr_config_location = f"./config/{user.screen_name}"
    logger.debug(f"[{get_time_fmt()}] Creating new user config at {usr_config_location}")

    with open(usr_config_location, "w") as outfile:
        outfile.write(auth.access_token)
        outfile.write("\n")
        outfile.write(auth.access_token_secret)
        outfile.write("\n")

else:
    with open(args.credentials_filename, "r") as infile:
        access_token, access_token_secret, *_ = infile.read().split("\n")

    auth.set_access_token(access_token, access_token_secret)
    api = tweepy.API(auth)

    logger.debug(f"[{get_time_fmt()}] Posting Tweet as @{api.me().screen_name}; " +
            f"file location: {args.tweet_filename}")

    try:
        with open(args.tweet_filename, 'r') as tweetfile:
            tweet_text = tweetfile.read().strip()
    except OSError as e:
        print("[ERROR]: Could not read from file.")
        logger.error(f"[{get_time_fmt()}] Could not read from file: {tweet_filename}; " +
                os.strerror(e.errno))
    else:
        if len(tweet_text) <= 280:
            api.update_status(tweet_text)
        else:
            print("[ERROR]: Tweet text too long!")
            logger.error(f"[{get_time_fmt()}] Tweet contents too long; argument length: " +
                    f"{len(tweet_text)}; file location: {args.tweet_filename}")

