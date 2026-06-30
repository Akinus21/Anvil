class Anvil < Formula
  desc "Modular CLI tool runner with module registry"
  homepage "https://github.com/Anvil/Anvil"
  version "1.00"
  url "https://github.com/Anvil/Anvil/releases/download/v1.00/Anvil-v1.00-linux-x86_64.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256_AFTER_RELEASE"

  def install
    bin.install "anvil"
  end

  test do
    system "#{bin}/anvil", "--version"
  end
end
