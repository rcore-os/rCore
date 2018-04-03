class Objconv < Formula
  desc "Object file converter and disassembler"
  homepage "http://www.agner.org/optimize/#objconv"
  url "http://www.agner.org/optimize/objconv.zip"
  version "2.44"
  sha256 "f2c0c4cd6ff227e76ffed5796953cd9ae9eb228847ca9a14dba6392c573bb7a4"
  def install
    system "unzip", "source.zip",
                    "-dsrc"
    # objconv doesn't have a Makefile, so we have to call
    # the C++ compiler ourselves
    system ENV.cxx, "-o", "objconv",
                    "-O2",
                    *Dir["src/*.cpp"],
                    "--prefix=#{prefix}"
    bin.install "objconv"
  end

  test do
    system "#{bin}/objconv", "-h"
    # TODO: write better tests
  end
end
