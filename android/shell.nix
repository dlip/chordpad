{ pkgs ? import <nixpkgs> { config = { allowUnfree = true; android_sdk.accept_license = true; }; } }:
let
  sdk = (pkgs.androidenv.composeAndroidPackages {
    platformVersions = [ "35" ];
    buildToolsVersions = [ "35.0.0" ];
    includeEmulator = false;
    includeSystemImages = false;
  }).androidsdk;
in pkgs.mkShell {
  packages = [ pkgs.jdk21 pkgs.gradle sdk ];
  JAVA_HOME = "${pkgs.jdk21}";
  ANDROID_HOME = "${sdk}/libexec/android-sdk";
  GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${sdk}/libexec/android-sdk/build-tools/35.0.0/aapt2";
}
