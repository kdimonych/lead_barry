#!/bin/bash

# This script must be run not from scripts/ but from the project root, e.g.:
# ./scripts/create_python_env.sh
# So, check if we're not in the scripts/ directory by looking if working directory
# contains the scripts/ directory. If it does, exit with an error message.
if [[ "$PWD" == *"/scripts" ]]; then
    echo "Please run this script from the project root, not from the scripts/ directory."
    echo "Example: ./scripts/create_python_env.sh"
    exit 1
fi

# Check if Python 3 is installed
if ! command -v python3 &> /dev/null; then
    echo "Python 3 is not installed. Please install Python 3 to continue."
    exit 1
fi

# Check if pip is installed
if ! command -v pip &> /dev/null; then
    echo "pip is not installed. Please install pip to continue."
    exit 1
fi

# Check if venv module is available
if ! python3 -m venv --help &> /dev/null; then
    echo "The venv module is not available. Please ensure you have Python 3.3 or higher installed."
    exit 1
fi

# Check if another virtual environment is already active. If so, deactivate it before creating a new one.
if [[ "$VIRTUAL_ENV" != "" ]]; then
    echo "The \"$VIRTUAL_ENV\" virtual environment is currently active. \
Deactivate it with \"deactivate\" command before running this script to create a new one."
    exit 1
fi

# Create a Python virtual environment and install dependencies
if [ -d ".venv" ]; then
    echo "Virtual environment already exists, skipping creation..."
else
  echo "Creating Python virtual environment..."
  python3 -m venv ./.venv
fi

# Add the environment dir to gitignore if it's not already there
if ! grep -q ".venv/" .gitignore; then
    echo "Adding .venv/ to .gitignore..."
    echo "" >> .gitignore
    echo "# Python virtual environment" >> .gitignore
    echo ".venv/" >> .gitignore
else
    echo ".venv/ already in .gitignore, skipping..."
fi

# Activate the virtual environment and install dependencies
echo "Activating virtual environment \"./.venv\" and installing dependencies..."
source ./.venv/bin/activate

# Check if requirements.txt exists before trying to install
if [ -f "requirements.txt" ]; then
    pip install -r requirements.txt
else
    echo "No requirements.txt found, skipping dependency installation."
fi
