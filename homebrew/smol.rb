# typed: false
# frozen_string_literal: true

class Smol < Formula
  desc "Smart Minimal Output Logger — run commands and get summarized output for LLMs"
  homepage "https://github.com/nnar1o/smol"
  url "https://github.com/nnar1o/smol/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "CHANGE_ME"  # Will be set after the next release tag
  license "MIT"
  version "0.1.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Test that smol binary works
    output = shell_output("#{bin}/smol --help")
    assert_match "smol", output

    # Test that smol can run a simple command
    output = shell_output("#{bin}/smol --sync echo hello")
    assert_match "success", output
  end
end
