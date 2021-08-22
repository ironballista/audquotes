#!/bin/bash

fname=quotes

stest -qvs "$fname" && find . -type f -iname "*.txt" | shuf >"$fname"

quote=$(head -1 "$fname")
tail -n +2 "$fname" > "$fname.tmp" && mv "$fname.tmp" "$fname"

tweet -c "./acc.toml" send "$(cat $quote)"

