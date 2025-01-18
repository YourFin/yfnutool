{
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "yfnutool";
  version = "0.1.0";

  src = ./.;
  cargoHash = "sha256-vvJnuJwmZIgoDHSR5/KQ8Iuh9FelKlqIy2PW8UjooS4=";
  postInstall = ''
    mkdir -p $out/share/nushell/vendor/autoload
    cp -r $src/nu-mod/yfnutool $out/share/nushell/vendor/autoload/yfnutool
  '';
}
