#!/usr/bin/env sh

# nohup influxd >/dev/null 2>&1 &
# exec ockamd "$@"

nohup influxd >/dev/null 2>&1 &
exec ockamd "$@"