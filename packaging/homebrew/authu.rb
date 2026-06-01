# Homebrew formula for `authu` — srun (深澜) campus-portal auth CLI.
#
# This file lives in this repo as the canonical template; the published
# formula lives in a separate `WangYihang/homebrew-tap` repo so users can
# `brew tap WangYihang/tap && brew install authu`. Run
# `scripts/bump-packaging.sh <version>` after each release to refresh the
# version + sha256 strings, then sync the file across.

class Authu < Formula
  desc "srun (深澜) campus-portal auth CLI — auto-discovers host + ac_id"
  homepage "https://github.com/WangYihang/authu"
  version "0.2.0"
  license "GPL-3.0-or-later"

  on_macos do
    on_arm do
      url "https://github.com/WangYihang/authu/releases/download/v#{version}/authu-aarch64-apple-darwin.tar.gz"
      sha256 "ff71c0ce6d30979a52e8c0b00e79cdfe793a84240e8f4247155ffec0e3d9a49c"
    end
    on_intel do
      url "https://github.com/WangYihang/authu/releases/download/v#{version}/authu-x86_64-apple-darwin.tar.gz"
      sha256 "7a65f50c5c06a9077fa231088c89e8379b9ab5700ce478208735b31564d7dd54"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/WangYihang/authu/releases/download/v#{version}/authu-aarch64-unknown-linux-musl.tar.gz"
      sha256 "2b768a12eb64d3a89a45574b9401d79f39ba80d5a73f7ac1e9c61fae2b97d119"
    end
    on_intel do
      url "https://github.com/WangYihang/authu/releases/download/v#{version}/authu-x86_64-unknown-linux-musl.tar.gz"
      sha256 "3bd467f929526ffac86353b7ca3c7ed0eb8c499a540ccc4312178c12ae029820"
    end
  end

  def install
    bin.install "authu"
    doc.install "README.md", "LICENSE"
    pkgshare.install Dir["docs/*"] if File.directory?("docs")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/authu --version")
  end
end
