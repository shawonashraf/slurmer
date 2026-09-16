#!/usr/bin/env bash
# Render the Homebrew cask for a slurmer release.
#
#   scripts/homebrew-cask.sh <version> <arm64 dmg sha256> <x64 dmg sha256>
#
# Writes the cask to stdout. The release workflow runs this after a release is
# published and commits the output to shawonashraf/homebrew-tap as
# Casks/slurmer.rb, so edit this template rather than the file in the tap.
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <version> <arm64-sha256> <x64-sha256>" >&2
  exit 2
fi

version="$1"
arm_sha="$2"
intel_sha="$3"

case "$version" in
  v*) echo "version must not start with 'v' (got '$version')" >&2; exit 2 ;;
esac
for sha in "$arm_sha" "$intel_sha"; do
  if ! [[ "$sha" =~ ^[0-9a-f]{64}$ ]]; then
    echo "not a sha256 hex digest: '$sha'" >&2
    exit 2
  fi
done

cat <<CASK
cask "slurmer" do
  arch arm: "aarch64", intel: "x64"

  version "${version}"
  sha256 arm:   "${arm_sha}",
         intel: "${intel_sha}"

  url "https://github.com/shawonashraf/slurmer/releases/download/v#{version}/slurmer_#{version}_#{arch}.dmg"
  name "slurmer"
  desc "Slurm job queue viewer for HPC clusters, fetched over ssh"
  homepage "https://github.com/shawonashraf/slurmer"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: :big_sur

  app "slurmer.app"

  # Release builds are ad-hoc signed and not notarized, so Gatekeeper reports
  # the app as damaged on first launch unless the quarantine flag is removed.
  postflight_steps do
    run "/usr/bin/xattr", args: ["-cr", "{{appdir}}/slurmer.app"]
  end

  uninstall quit: "org.shawonashraf.slurmer"

  zap trash: [
    "~/Library/Application Support/org.shawonashraf.slurmer",
    "~/Library/Caches/slurmer",
    "~/Library/Saved Application State/org.shawonashraf.slurmer.savedState",
    "~/Library/WebKit/slurmer",
  ]

  caveats <<~EOS
    slurmer is not notarized. This cask removes the quarantine attribute from
    slurmer.app after installing, so macOS does not report it as damaged.
  EOS
end
CASK
