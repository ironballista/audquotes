#!/bin/bash

while [[ $# -gt 0 ]]; do
    key="$1"

    case $key in
    -t|--tweet)
        TWEETMODE=ON
        shift # shift past argument
        ;;
    -f|--filename)
        FNAME="$2"
        shift # shift past argument
        shift # shift past value
        ;;
    esac
done

if [ -z ${FNAME} ]; then
    FNAME=quotes
fi

stest -qvs "${FNAME}" && find . -type f -iname "*.txt" | shuf >"${FNAME}"

QUOTE=$(head -1 "${FNAME}")
tail -n +2 "${FNAME}" > "${FNAME}.tmp" && mv "${FNAME}.tmp" "${FNAME}"

if [ ${TWEETMODE} ]; then
    tweet -c "./acc.toml" send "$(cat ${QUOTE})"
else
    cat ${QUOTE}
fi

