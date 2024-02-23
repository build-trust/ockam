import os
import re

def delete_first_match(filename):
    with open(filename, 'r') as file:
        content = file.read()

    regex_pattern = r'## \d+\.\d+\.\d+.*\n+(?:### .*\n+(?:-.*\n+)+)+'
    match = re.search(regex_pattern, content)

    if match:
        pruned_content = content.replace(match.group(), '', 1)

        with open(filename, 'w') as file:
            file.write(pruned_content)

file_path = os.environ['CHANGELOG_FILE_PATH']
delete_first_match(file_path)
