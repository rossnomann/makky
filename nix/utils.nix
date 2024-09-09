{ lib }:
let
  safeCharactersList =
    [
      "+"
      "."
      "_"
      "?"
      "="
    ]
    ++ lib.strings.lowerChars ++ lib.strings.upperChars ++ lib.strings.stringToCharacters "0123456789";
  mkReplacementCharactersList = l: lib.genList (x: "") (lib.length l);
  replacementSafeCharactersList = mkReplacementCharactersList safeCharactersList;
  mkStoreName =
    prefix: value:
    let
      unsafeString = lib.strings.replaceStrings safeCharactersList replacementSafeCharactersList value;
      unsafeCharactersList = lib.strings.stringToCharacters unsafeString;
      replacementUnsafeCharactersList = mkReplacementCharactersList unsafeCharactersList;
      result = lib.strings.replaceStrings unsafeCharactersList replacementUnsafeCharactersList value;
    in
    prefix + result;
  mkStorePath =
    prefix: source:
    let
      sourceString = toString source;
    in
    if builtins.hasContext sourceString then
      source
    else
      builtins.path {
        path = source;
        name = mkStoreName prefix (baseNameOf sourceString);
      };
in
{
  inherit mkStoreName mkStorePath;
}
