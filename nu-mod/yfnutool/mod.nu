export def "yfnutool interpolate" [] {
  let cli = commandline
  let pos = commandline get-cursor
  let result = [pos, cli] | to msgpack | _yfnutool-bin | from msgpack
  commandline edit --replace $result.1
  commandline set-cursor $result.0
}
