#!/usr/bin/env python3

import sys
import csv

pr_sender = sys.argv[1]
has_accepted_cla = False
contributors_file = sys.stdin.readlines()

for line in csv.reader(contributors_file):
  if line[1] == pr_sender:
    has_accepted_cla = True
    break

if not has_accepted_cla:
  message = """
  Hi {}, welcome to the Ockam community and thank you for sending this pull request ❤️.

  Before we can merge, please accept our Contributor License Agreement (CLA).

  1. Read the CLA at: https://www.ockam.io/learn/how-to-guides/contributing/cla

  2. To accept the CLA, please send a different pull request to our
  [contributors repository](https://github.com/ockam-network/contributors) indicating that you accept
  the CLA by adding your Git/Github details in a row at the end of the
  [CONTRIBUTORS.csv](https://github.com/ockam-network/contributors/blob/main/CONTRIBUTORS.csv) file.

  We look forward to merging your first contribution!
  """.format(pr_sender)
  print(message, file=sys.stderr)
  sys.exit(1)

print("[✓] {} has accepted the Ockam CLA.".format(pr_sender))
