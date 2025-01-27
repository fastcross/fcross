#!/bin/bash
set -x
set -e

killall wasmd || true
killall rly || true