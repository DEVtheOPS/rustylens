#!/usr/bin/env node

/**
 * Generates update.json for Tauri updater from GitHub release assets.
 */

import { writeFileSync } from 'fs';

const GITHUB_REPO = 'DEVtheOPS/kore';
const GITHUB_TOKEN = process.env.GITHUB_TOKEN;
const TAG_NAME = process.env.TAG_NAME;

if (!GITHUB_TOKEN) {
  console.error('GITHUB_TOKEN environment variable is required');
  process.exit(1);
}

if (!TAG_NAME) {
  console.error('TAG_NAME environment variable is required');
  process.exit(1);
}

async function fetchRelease() {
  const response = await fetch(
    `https://api.github.com/repos/${GITHUB_REPO}/releases/tags/${TAG_NAME}`,
    {
      headers: {
        Authorization: `Bearer ${GITHUB_TOKEN}`,
        Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28'
      }
    }
  );

  if (!response.ok) {
    throw new Error(`Failed to fetch release: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

async function fetchSignature(url) {
  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${GITHUB_TOKEN}`,
      Accept: 'application/octet-stream'
    }
  });

  if (!response.ok) {
    console.warn(`Failed to fetch signature from ${url}: ${response.status}`);
    return '';
  }

  return response.text();
}

async function generateUpdateJson() {
  console.log(`Fetching release ${TAG_NAME}...`);
  const release = await fetchRelease();

  const version = TAG_NAME.replace(/^v/, '');
  const notes = release.body || `Release ${version}`;
  const pubDate = release.published_at;
  const assets = release.assets;

  console.log(`Found ${assets.length} assets`);

  const platforms = {
    'darwin-aarch64': { urlPattern: /aarch64-apple-darwin.*\.tar\.gz$/, sigPattern: /aarch64-apple-darwin.*\.tar\.gz\.sig$/ },
    'darwin-x86_64': { urlPattern: /x86_64-apple-darwin.*\.tar\.gz$/, sigPattern: /x86_64-apple-darwin.*\.tar\.gz\.sig$/ },
    'linux-x86_64': { urlPattern: /\.AppImage\.tar\.gz$/, sigPattern: /\.AppImage\.tar\.gz\.sig$/ },
    'windows-x86_64': { urlPattern: /\.msi\.zip$/, sigPattern: /\.msi\.zip\.sig$/ }
  };

  const platformEntries = {};

  for (const [platform, config] of Object.entries(platforms)) {
    const urlAsset = assets.find((a) => config.urlPattern.test(a.name));
    const sigAsset = assets.find((a) => config.sigPattern.test(a.name));

    if (!urlAsset) {
      console.warn(`No asset found for ${platform}`);
      continue;
    }

    let signature = '';
    if (sigAsset) {
      signature = await fetchSignature(sigAsset.browser_download_url);
    }

    platformEntries[platform] = {
      signature: signature.trim(),
      url: urlAsset.browser_download_url
    };

    console.log(`Added ${platform}: ${urlAsset.name}`);
  }

  const updateJson = {
    version,
    notes,
    pub_date: pubDate,
    platforms: platformEntries
  };

  writeFileSync('update.json', JSON.stringify(updateJson, null, 2));
  console.log('Generated update.json');
}

generateUpdateJson().catch((err) => {
  console.error('Error:', err);
  process.exit(1);
});
