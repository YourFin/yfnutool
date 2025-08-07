{
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "yfnutool";
  version = "0.1.0";

  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  postInstall = ''
    mkdir -p $out/share/nushell/vendor/autoload
    cp -r $src/nu-mod/yfnutool $out/share/nushell/vendor/autoload/yfnutool
  '';
}
