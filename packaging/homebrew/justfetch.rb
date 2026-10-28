# Draft formula. NOT PUBLISHED.
#
# The version and the sha256 values must come from the real release assets:
# download each archive, run `shasum -a 256`, and copy the value from the
# release's SHA256SUMS. Do not publish with placeholders.
class Justfetch < Formula
  desc "Fast, offline system information in the terminal or as JSON"
  homepage "https://github.com/LightYourJourney/justfetch"
  version "1.0.0-rc.1"
  license "GPL-3.0-only"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/LightYourJourney/justfetch/releases/download/v#{version}/justfetch-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_AARCH64_APPLE_DARWIN"
    else
      url "https://github.com/LightYourJourney/justfetch/releases/download/v#{version}/justfetch-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/LightYourJourney/justfetch/releases/download/v#{version}/justfetch-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_AARCH64_LINUX"
    else
      url "https://github.com/LightYourJourney/justfetch/releases/download/v#{version}/justfetch-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_X86_64_LINUX"
    end
  end

  def install
    bin.install "justfetch"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/justfetch --version")
    assert_match '"schema_version": 1', shell_output("#{bin}/justfetch --format json")
  end
end
