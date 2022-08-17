# Pipe set -x log to a file https://stackoverflow.com/questions/25593034/capture-x-debug-commands-into-a-file-in-bash
log=$(mktemp)
echo "Log directory is $log"

exec 5> $log
BASH_XTRACEFD="5"