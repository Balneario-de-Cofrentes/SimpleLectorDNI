# Host detection shared by the packaging scripts. Sets:
#   platform     macos | windows
#   architecture arm64 | x64
#   exe          "" or ".exe" (binary suffix)
#   jlink        jlink or jlink.exe
case "$(uname -s)" in
  Darwin)
    platform=macos
    exe=""
    jlink=jlink
    case "$(uname -m)" in
      arm64) architecture=arm64 ;;
      x86_64) architecture=x64 ;;
      *) echo "Unsupported macOS architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    platform=windows
    architecture=x64
    exe=".exe"
    jlink=jlink.exe
    ;;
  *) echo "Unsupported platform: $(uname -s)" >&2; exit 1 ;;
esac

# Builds the trimmed Java runtime into $1 from scripts/runtime-modules.txt.
build_runtime() {
  runtime_modules=$(tr -d '\r\n' < scripts/runtime-modules.txt)
  "$JAVA_HOME/bin/$jlink" \
    --add-modules "$runtime_modules" \
    --compress zip-6 \
    --strip-debug \
    --no-header-files \
    --no-man-pages \
    --output "$1"
}
