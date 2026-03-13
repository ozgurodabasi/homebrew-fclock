class Fclock < Formula
  desc "Full-screen terminal clock and countdown timer with Matrix rain"
  homepage "https://github.com/ozgurodabasi/homebrew-fclock"
  url "https://github.com/ozgurodabasi/homebrew-fclock/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "434d74660e28e9bf5c02081d7c204c25171d52a5b75dc145c7d24dd3e5c3ab3e"
  license "MIT"
  head "https://github.com/ozgurodabasi/homebrew-fclock.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_predicate bin/"fclock", :exist?
  end
end
