#!/bin/bash

if [ "$MERGE_CHANGELOG" = true ]; then
  echo "$(rclog -t $NEW_VERSION -p ./CHANGELOG.md merge)" >| ./CHANGELOG.md
fi
