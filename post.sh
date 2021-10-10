#!/bin/bash

while [[ $# -gt 0 ]]; do
    key="$1"

    case $key in
    -h|--help)
        echo -e "$0 - usage:" \
             "[-h] [-c CREDENTIALS] [-f POSTLIST] [-t]\n" \
             "-h, --help             show this message\n" \
             "-t, --tweet            enables tweeting mode\n" \
             "-f, --filename         file containing post list\n" \
             "-c, --credentials      specify twitter credentials file"
        exit
        ;;
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
    CREDENTIALS="./config/acc.toml"
fi

if [ -z ${FNAME} ]; then
    FNAME="./config/quotes"
fi

! test -s "${FNAME}" && find "./src" -type f -iname "*.txt" | shuf >"${FNAME}"

QUOTE_FNAME=$(head -1 "${FNAME}")
tail -n +2 "${FNAME}" > "${FNAME}.tmp" && mv "${FNAME}.tmp" "${FNAME}"

if [ ${TWEETMODE} ]; then
    ./util/tweet.py -c "${CREDENTIALS}" send -f "${QUOTE_FNAME}"
else
    cat ${QUOTE_FNAME}
fi

