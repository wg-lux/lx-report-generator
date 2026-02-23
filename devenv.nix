{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  languages.rust.enable = true;

  # https://devenv.sh/packages/
  packages = [ 
    pkgs.git
    pkgs.cargo
    pkgs.rustc
     ];

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';

  scripts.ensure_report_renderer.exec = ''
    set -eu
    BIN_PATH="$PWD/target/release/report_pdf_renderer"
    if [ -x "$BIN_PATH" ]; then
      echo "report_pdf_renderer already built: $BIN_PATH"
      exit 0
    fi

    echo "Bootstrapping report_pdf_renderer (release build)..."
    cargo build --release
    echo "Built: $BIN_PATH"
    echo "Production runtime env var:"
    echo "  export ENDOREG_REPORT_PDF_RENDERER_BIN=$BIN_PATH"
  '';

  # https://devenv.sh/basics/
  enterShell = ''
    hello         # Run scripts directly
    git --version # Use packages
  '';

  # https://devenv.sh/tasks/
  tasks = {
    "report-renderer:bootstrap" = {
      exec = "ensure_report_renderer";
      status = "test -x ./target/release/report_pdf_renderer";
    };
    "devenv:enterShell".after = [ "report-renderer:bootstrap" ];
  };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
