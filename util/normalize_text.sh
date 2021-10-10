#!/bin/bash

# While the length is non-zero
while [ -n "$1" ]; do
    sed 's/…/.../g' "$1" | sed "s/’/'/g" | sed -E 's/“|”/"/g' | sed -E 's/\ \ +//g' >.buff
    mv .buff "$1"
    shift
done

exit
