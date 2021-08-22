#!/bin/bash

while [[ $# -gt 0 ]]; do
    key="$1"

    case $key in
    -t|--tweet)
        # Enables tweeting of the text files
        # as opposed to simply displaying them
        TWEETMODE=ON
        shift # past argument
        ;;
    -f|--filename)
        # Controls which file contains the list
        # of filenames to select
        FNAME="$2"
        shift # past argument
        shift # past value
        ;;
    -c|--credentials)
        # Optionally specify twitter credentials file
        CREDENTIALS="$2"
        shift # past argument
        shift # past value
        ;;
    esac
done

if [ -z ${CREDENTIALS} ]; then
    CREDENTIALS=acc.toml
fi

if [ -z ${FNAME} ]; then
    FNAME=quotes
fi

stest -qvs "${FNAME}" && find . -type f -iname "*.txt" | shuf >"${FNAME}"

QUOTE=$(head -1 "${FNAME}")
tail -n +2 "${FNAME}" > "${FNAME}.tmp" && mv "${FNAME}.tmp" "${FNAME}"

if [ ${TWEETMODE} ]; then
    tweet -c "${CREDENTIALS}" send "$(cat ${QUOTE})"
else
    cat ${QUOTE}
fi

