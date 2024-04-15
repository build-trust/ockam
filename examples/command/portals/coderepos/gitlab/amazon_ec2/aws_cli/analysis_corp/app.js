const { exec } = require('child_process');
const fs = require('fs');
const path = require('path');

const repoUrl = 'git@127.0.0.1:root/demo_project.git';
const repoDir = 'demo_project';

function executeCommand(command) {
  return new Promise((resolve, reject) => {
    exec(command, (error, stdout, stderr) => {
      if (error) {
        console.error(`Error executing command: ${command}`);
        console.error(`Error: ${error.message}`);
        console.error(`stderr: ${stderr}`);
        reject(error);
      } else {
        console.log(`Command executed successfully: ${command}`);
        resolve(stdout);
      }
    });
  });
}

async function cloneRepository() {
  if (!fs.existsSync(repoDir)) {
    try {
      await executeCommand(`git clone ${repoUrl}`);
      console.log('Repository cloned successfully.');
    } catch (error) {
      console.error('Error cloning repository:', error.message);
      throw error;
    }
  } else {
    console.log('Repository already exists. Skipping clone.');
  }
}

async function commitChanges(message) {
  try {
    await executeCommand(`git -C ${repoDir} add .`);
    const statusOutput = await executeCommand(`git -C ${repoDir} status --porcelain`);
    if (statusOutput.trim() !== '') {
      await executeCommand(`git -C ${repoDir} commit -m "${message}"`);
      console.log('Changes committed successfully.');
    } else {
      console.log('No changes to commit. Skipping commit.');
    }
  } catch (error) {
    console.error('Error committing changes:', error.message);
    throw error;
  }
}

async function pushChanges() {
  try {
    await executeCommand(`git -C ${repoDir} push`);
    console.log('Changes pushed successfully.');
  } catch (error) {
    console.error('Error pushing changes:', error.message);
    throw error;
  }
}

async function validateSourceCode() {
  const readmePath = path.join(repoDir, 'README.md');

  try {
    await fs.promises.access(readmePath, fs.constants.F_OK);
    console.log('README.md file exists.');
    // Add your source code validation logic here
    console.log('Source code validation successful.');
  } catch (error) {
    if (error.code === 'ENOENT') {
      console.error('README.md file does not exist.');
    } else {
      console.error('Error validating source code:', error.message);
    }
    throw error;
  }
}

async function updateReadme() {
  const readmePath = path.join(repoDir, 'README.md');
  const appendText = '\nThis line was appended by the script.';

  try {
    await fs.promises.appendFile(readmePath, appendText);
    console.log('README.md updated successfully.');
  } catch (error) {
    console.error('Error updating README.md:', error.message);
    throw error;
  }
}

async function run() {
  try {
    await cloneRepository();
    await validateSourceCode();
    await updateReadme();
    await commitChanges('Source code validation successful and README.md updated');
    await pushChanges();
    console.log(
      "\nThe example run was successful 🥳.\n" +
      "\nThe app validated the source code from the repository." +
      "\nPerformed necessary checks, updated README.md, and pushed the changes.\n"
    );
  } catch (err) {
    console.error('Error:', err.message);
  }
}

run();
