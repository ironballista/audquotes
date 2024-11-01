#!/bin/bash

parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

while [[ $# -gt 0 ]]; do
    key="$1"

    case $key in
    -h|--help)
        echo -e "$0 - usage:" \
             "[-h] [-c CREDENTIALS] [-f POSTLIST] [-t]\n" \
             "-h, --help             show this message\n" \
             "-t, --tweet            enables tweeting mode\n" \
             "-b, --bluesky          enables bluesky mode\n"
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
    -b|--bluesky)
        # Enables posting of text files via Bluesky
        BLUESKY=ON
        shift
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

function shift_list() {
    ! test -s "${FNAME}" && find "./src" -type f -iname "*.txt" | shuf >"${FNAME}"

    QUOTE_FNAME=$(head -1 "${FNAME}")
    tail -n +2 "${FNAME}" > "${FNAME}.tmp" && mv "${FNAME}.tmp" "${FNAME}"
}

function post_tweet() {
    ./util/tweet.py -c "${CREDENTIALS}" send -f "${QUOTE_FNAME}"
    return $?
}

function post_bluesky() {
    python3 ./util/bluesky.py -f "${QUOTE_FNAME}"
    return $?
}

shift_list

if [ ${TWEETMODE} ]; then
    # Keep going until a tweet was posted
    while post_tweet; [ $? -ne 0 ]; do shift_list; done
elif [ ${BLUESKY} ]; then
    # Keep going until a post is successful
    while post_bluesky; [ $? -ne 0 ]; do shift_list; done
else
    cat ${QUOTE_FNAME}
fi
