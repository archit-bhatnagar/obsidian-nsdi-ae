#!/bin/bash

# Define the folder path
PERSISTENCE_FOLDER="Persistence"

# Check if the folder exists
if [ -d "$PERSISTENCE_FOLDER" ]; then
  # Delete all files within the Persistence folder
  rm -f "$PERSISTENCE_FOLDER"/*

  # Check if the delete operation was successful
  if [ $? -eq 0 ]; then
    echo "All files in the $PERSISTENCE_FOLDER folder have been deleted."
  else
    echo "Failed to delete files in the $PERSISTENCE_FOLDER folder."
  fi
else
  echo "Folder $PERSISTENCE_FOLDER does not exist."
fi
