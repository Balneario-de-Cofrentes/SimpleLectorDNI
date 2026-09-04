$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

& "$PSScriptRoot/build-worker.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo build --release --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$Version = $Metadata.packages[0].version
if ($env:GITHUB_REF_TYPE -eq "tag" -and $env:GITHUB_REF_NAME -ne "v$Version") {
    throw "Release tag $env:GITHUB_REF_NAME does not match Cargo version $Version"
}
$PackageName = "SimpleLectorDNI-v$Version-windows-x64"
$TemporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("simple-lector-dni-" + [guid]::NewGuid())
$PackageDir = Join-Path $TemporaryRoot $PackageName
$OutputArchive = "dist/$PackageName.zip"

try {
    New-Item -ItemType Directory -Force -Path "$PackageDir/engine" | Out-Null
    Copy-Item target/release/simple-lector-dni.exe "$PackageDir/simple-lector-dni.exe"
    Copy-Item engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$PackageDir/engine/"
    foreach ($Source in (Get-Content "$PSScriptRoot/release-files.txt")) {
        if ([string]::IsNullOrWhiteSpace($Source)) { continue }
        $Destination = Join-Path $PackageDir $Source
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
        Copy-Item $Source $Destination
    }
    $RuntimeModules = (Get-Content "$PSScriptRoot/runtime-modules.txt" -Raw).Trim()

    & "$env:JAVA_HOME/bin/jlink.exe" `
        --add-modules $RuntimeModules `
        --compress zip-6 `
        --strip-debug `
        --no-header-files `
        --no-man-pages `
        --output "$PackageDir/runtime"
    if ($LASTEXITCODE -ne 0) { throw "jlink failed with exit code $LASTEXITCODE" }

    & "$PackageDir/simple-lector-dni.exe" --version | Out-Null
    $WorkerResponse = '{"protocol":99,"command":"read","reader_name":"Synthetic reader"}' |
        & "$PackageDir/runtime/bin/java.exe" -jar "$PackageDir/engine/simple-lector-dni-engine.jar" |
        ConvertFrom-Json
    if ($WorkerResponse.error.code -ne "INVALID_REQUEST") { throw "Packaged worker failed" }
    $ExpectedJavaVersion = ((Get-Content "$PackageDir/.java-version" -Raw).Trim() -split '\+')[0]
    $RuntimeRelease = Get-Content "$PackageDir/runtime/release"
    if ($RuntimeRelease -notcontains "JAVA_VERSION=`"$ExpectedJavaVersion`"") {
        throw "Packaged runtime does not match .java-version"
    }

    New-Item -ItemType Directory -Force -Path dist | Out-Null
    $TemporaryArchive = Join-Path $TemporaryRoot "$PackageName.zip"
    Compress-Archive -Path $PackageDir -DestinationPath $TemporaryArchive
    Move-Item -Force $TemporaryArchive $OutputArchive
}
finally {
    if (Test-Path $TemporaryRoot) {
        Remove-Item -Recurse -Force $TemporaryRoot
    }
}
Write-Output $OutputArchive
