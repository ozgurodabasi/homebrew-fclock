class Fclock < Formula
  desc "Full-screen terminal clock and countdown timer with Matrix rain"
  homepage "https://github.com/ozgurodabasi/homebrew-fclock"
  url "https://github.com/ozgurodabasi/homebrew-fclock/archive/refs/tags/v0.1.3.tar.gz"
  sha256 "a515ea211ce2eef053e5b5dcf7846e2f28034fad6d5b49488b9e2927965cc02c"
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
