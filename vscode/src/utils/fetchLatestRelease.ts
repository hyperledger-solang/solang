import fetch from 'node-fetch';

export default async function fetchLatestRelease() {
  const RELEASE_URL = 'https://api.github.com/repos/hyperledger/solang/releases/latest';
  const response = await fetch(RELEASE_URL);

  if (!response.ok) {
    console.error('Error fetching artifact release info');

    throw new Error(`Got response ${response.status} when trying to fetch release info`);
  }

  const release = await response.json();
  return release;
}
