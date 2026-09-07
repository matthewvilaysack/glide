#!/usr/bin/env bash
# Render Formula/glide.rb for the tap from a release's checksums.
# usage: scripts/homebrew-formula.sh <version> <dir with .sha256 files or checksums.txt>
set -euo pipefail
version="${1:?version}"
dir="${2:?dir}"
repo="matthewvilaysack/glide"
base="https://github.com/$repo/releases/download/v$version"

sum() {
  local target="$1"
  local file="glide-$version-$target.tar.gz"
  if [ -f "$dir/checksums.txt" ]; then
    grep " $file\$" "$dir/checksums.txt" | cut -d' ' -f1
  else
    cut -d' ' -f1 < "$dir/$file.sha256"
  fi
}

cat <<RUBY
class Glide < Formula
  desc "Keep your priorities in view while you work in the terminal; onboarding agent for engineering teams"
  homepage "https://tryglide.dev"
  version "$version"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "$base/glide-$version-aarch64-apple-darwin.tar.gz"
      sha256 "$(sum aarch64-apple-darwin)"
    end
    on_intel do
      url "$base/glide-$version-x86_64-apple-darwin.tar.gz"
      sha256 "$(sum x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "$base/glide-$version-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "$(sum aarch64-unknown-linux-gnu)"
    end
    on_intel do
      url "$base/glide-$version-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$(sum x86_64-unknown-linux-gnu)"
    end
  end

  def install
    bin.install "glide"
    generate_completions_from_executable(bin/"glide", "completion")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/glide --version")
  end
end
RUBY
