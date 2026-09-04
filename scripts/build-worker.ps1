$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot
$Maven = if ($env:MVN) { $env:MVN } else { "mvn" }

git submodule update --init --recursive
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& $Maven -q -f vendor/jmulticard/pom.xml -DskipTests install
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& $Maven -q -f engine/jmulticard-worker/pom.xml package
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Response = '{"protocol":99,"command":"read","reader_index":0}' |
    & "$env:JAVA_HOME/bin/java.exe" -jar engine/jmulticard-worker/target/simple-lector-dni-engine.jar |
    ConvertFrom-Json

if ($Response.status -ne "error" -or $Response.error.code -ne "INVALID_REQUEST") {
    throw "Worker package smoke test failed"
}
