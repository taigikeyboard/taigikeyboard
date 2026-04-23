#!/bin/bash
set -e
cd "$(dirname "$0")"
PYTHONPATH="$(pwd)" python3 -m pipeline.run
