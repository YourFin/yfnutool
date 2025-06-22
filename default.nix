{
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "yfnutool";
  version = "0.1.0";

  src = ./.;
  cargoHash = "sha256-69g/Jau2Yz7kvJGHqGv8Nf5qCgcJhXCcup1evoLmry4=";
  postInstall = ''
    mkdir -p $out/share/nushell/vendor/autoload
    cp -r $src/nu-mod/yfnutool $out/share/nushell/vendor/autoload/yfnutool
  '';
}
