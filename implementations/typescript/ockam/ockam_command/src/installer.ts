import fs from "fs";
import https from "https";
import os from "os";
import path from "path";

async function download(url: string, downloadPath: string): Promise<void> {
  await fs.promises.mkdir(path.dirname(downloadPath), { recursive: true });

  let maxRedirects = 3;
  const makeRequest = async (url: string): Promise<void> => {
    return new Promise<void>((resolve, reject) => {
      const request = https.request(url, (response) => {
        if (
          response.statusCode &&
          response.statusCode >= 300 &&
          response.statusCode < 400 &&
          response.headers.location
        ) {
          if (maxRedirects-- < 0) {
            reject(new Error("Too many redirects"));
            return;
          }

          const redirectUrl = new URL(response.headers.location, url);
          resolve(makeRequest(redirectUrl.href));
          return;
        }

        if (response.statusCode && response.statusCode !== 200) {
          let err = `Failed to download, Status code: ${response.statusCode}`;
          reject(new Error(err));
          return;
        }

        const file = fs.createWriteStream(downloadPath);
        response.pipe(file);
        file.on("close", async () => {
          // give the downloaded file permission: -rwx------
          await fs.promises.chmod(downloadPath, 0o700);
          resolve();
        });
      });

      request.on("error", reject);
      request.end();
    });
  };

  return makeRequest(url);
}

function releaseFileName(platform: string, arch: string) {
  const releaseFileTypes: Record<string, Record<string, string>> = {
    darwin: {
      x64: "x86_64-apple-darwin",
      arm64: "aarch64-apple-darwin",
    },
    linux: {
      x64: "x64-unknown-linux-musl",
      arm64: "arm64-unknown-linux-musl",
      arm: "armv7l-unknown-linux-musleabihf",
    },
  };

  const fileType = releaseFileTypes[platform]?.[arch];
  if (!fileType) {
    let err = `Unsupported: Platform ${platform}, Arch ${arch}`;
    throw new Error(err);
  }

  return "ockam." + fileType;
}

export async function uninstall(home: string = "install") {
  return await fs.promises.rm(home, { recursive: true, force: true });
}

export async function isInstalled(home: string = "install"): Promise<boolean> {
  try {
    let binaryPath = path.join(home, "bin", "ockam");
    await fs.promises.access(binaryPath, fs.constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

export async function install(
  version: string = "latest",
  home: string = "install",
) {
  try {
    const fileToDownload = releaseFileName(process.platform, process.arch);
    const downloadPath = path.join(home, "bin", "ockam");
    const urlBase = "https://github.com/build-trust/ockam/releases";
    const url =
      version === "latest"
        ? `${urlBase}/latest/download/${fileToDownload}`
        : `${urlBase}/download/ockam_${version}/${fileToDownload}`;

    await download(url, downloadPath);
    return true;
  } catch (error) {
    console.error(`Error: ${error}`);
  }

  return false;
}
