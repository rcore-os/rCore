class Grub < Formula
  desc "GNU GRUB 2 targetting i386-elf"
  homepage "https://www.gnu.org/software/grub/"
  url "https://ftp.gnu.org/gnu/grub/grub-2.02.tar.xz"
  version "2.02"
  sha256 "810b3798d316394f94096ec2797909dbf23c858e48f7b3830826b8daa06b7b0f"

  depends_on "i386-elf-gcc"

  def install
    mkdir "grub-build" do
      system "../configure",
        "--disable-nls",
        "--disable-werror",
        "--disable-efiemu",
        "--target=i386-elf",
        "--prefix=#{prefix}",
        "TARGET_CC=i386-elf-gcc",
        "TARGET_NM=i386-elf-nm",
        "TARGET_OBJCOPY=i386-elf-objcopy",
        "TARGET_RANLIB=i386-elf-ranlib",
        "TARGET_STRIP=i386-elf-strip"

      system "make", "install"
    end
  end

  test do
    system "grub-shell", "--version"
  end
end