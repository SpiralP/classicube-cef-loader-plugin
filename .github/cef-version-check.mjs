#!/usr/bin/env zx

/** @type Record<string, {versions: Array<{cef_version: string, channel: "stable"|"beta", files: Array<{type: "standard"|"minimal"|"client", name: string, sha1: string}>}>}> */
const versionsByOs = await (
  await fetch("https://cef-builds.spotifycdn.com/index.json")
).json();

const mainPlatform = "windows64";
// linux32, linux64, linuxarm, linuxarm64, macosarm64, macosx64, windows32, windows64, windowsarm64
const requiredPlatforms = ["windows64", "linux64", "macosx64"];

function getCurrentVersion() {
  const content = fs
    .readFileSync(path.join(__dirname, "../src/cef_binary_updater.rs"))
    .toString();
  const match = content.match(/macro_rules! cef_version.+?"(.+?)"/s);
  if (match) {
    return match[1];
  }

  return null;
}

function getLatestStableVersion() {
  for (const version of versionsByOs[mainPlatform].versions) {
    const { cef_version, channel } = version;

    let ok = true;
    if (
      // skip beta versions
      version.channel !== "stable" ||
      version.files.some(({ name }) => name.includes("_beta"))
    ) {
      ok = false;
      console.warn("skipping beta version", { version });
    }
    for (const platform of requiredPlatforms) {
      if (
        !versionsByOs[platform].versions.find(
          (o) => o.cef_version === cef_version
        )
      ) {
        ok = false;
        console.warn("skipping version not found for required platform", {
          platform,
          cef_version,
        });
      }
    }
    if (ok) {
      return version;
    }
  }

  return null;
}

function main() {
  const currentVersion = getCurrentVersion();
  if (!currentVersion) {
    console.warn("!currentVersion");
    process.exit(1);
    return;
  }

  const latestVersion = getLatestStableVersion();
  if (!latestVersion) {
    console.warn("!latestVersion");
    process.exit(1);
    return;
  }

  const ok = currentVersion === latestVersion.cef_version;
  if (!ok) {
    console.log(latestVersion.cef_version);
    process.exit(1);
  } else {
    process.exit(0);
  }
}

main();
