# Aud Quotes

This repository hosts a collection of my favorite quotes from various pieces of media, as well as some scripts I use to manage them or post them to social media websites.

## File Structure

Each subfolder is designated with a specific purpose:

- `src/`: This directory contains **every** quote added to the repository in the form of **`.txt`** files. Any `.txt` file within this subdirectory is therefore considered a quote by all of the utilities included in the repository, **regardless of its nesting** into any subdirectories. Files whose names end in **any** other extension are ignored.
    - My personal organization for this folder is to dedicate a unique subfolder to each piece of media or series a set of quotes share in common (e.g. `src/ai-tsf` for the "AI: The Somnium Files" game or `src/zero-escape` for the Zero Escape series).
    - This is entirely arbitrary, so any eventual forks are free to adopt the file structure they prefer, so long as quotes are placed under `src/` and end in `.txt`.
- `util/`: This directory contains various **utility scripts** to manage the quotes stored in the repository.
- `schedule_script.py`: This file is the script meant to be executed on a server to periodically post random quotes sampled from `src/` at set time intervals. Currently, it assumes all posts are to be sent to [Bluesky](https://blueskyweb.zendesk.com/hc/en-us/articles/19002666608397-What-is-Bluesky) only.

> Note that not all files in this repository are currently documented. This has been a personal, private project for a few years and I have used multiple different tools and platforms with it, meaning some scripts present have been abandoned or remain outdated or obsolete.

