{
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "yfnutool";
  version = "0.1.0";

  src = ./.;
  cargoHash = "sha256-Y0vI1Hm9fXBsQDKxpR/s1d05D4gP78ZF+nTmaB4Vmos";
  postInstall = ''
    mkdir -p $out/share/nushell/vendor/autoload
    cp -r $src/nu-mod/yfnutool $out/share/nushell/vendor/autoload/yfnutool
  '';
}
