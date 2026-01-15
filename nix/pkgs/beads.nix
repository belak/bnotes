{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:
buildGoModule rec {
  pname = "beads";
  version = "0.47.1";

  src = fetchFromGitHub {
    owner = "steveyegge";
    repo = "beads";
    rev = "v${version}";
    hash = "sha256-DwIR/r1TJnpVd/CT1E2OTkAjU7k9/KHbcVwg5zziFVg=";
  };

  vendorHash = "sha256-pY5m5ODRgqghyELRwwxOr+xlW41gtJWLXaW53GlLaFw=";

  flags = [
    #"-trimpath"
  ];

  ldflags = [
    "-s"
    "-w"
    "-X main.Version=${version}"
    "-X main.Build=bnotes-nix"
  ];

  subPackages = [ "cmd/bd" ];

  doCheck = false;

  meta.mainProgram = "bd";
}
