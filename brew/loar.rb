class LOAR < Formula
  desc "LOAR - Local Archive Utility"
  homepage "https://github.com/cavecafe-cc/loar"
  url "https://github.com/cavecafe-cc/loar/releases/latest/loar_osx-arm64.tar.gz"
  sha256 "aec0d5dcd1680539272d742a433b97c4e86f8838c3dc07c84ba547a8437adec6"
  version "0.7.0"

  def install
    bin.install "loar"
  end
end