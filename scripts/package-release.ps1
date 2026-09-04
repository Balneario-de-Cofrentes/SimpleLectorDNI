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

New-Item -ItemType Directory -Force -Path "$PackageDir/engine", "$PackageDir/protocol" | Out-Null
Copy-Item target/release/simple-lector-dni.exe "$PackageDir/simple-lector-dni.exe"
Copy-Item engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$PackageDir/engine/"
Copy-Item protocol/engine-v1.schema.json "$PackageDir/protocol/"
Copy-Item .java-version, README.md, LICENSE, RUNTIME_SOURCE.md, THIRD_PARTY_NOTICES.md, THIRD_PARTY_LICENSES.md, THIRD_PARTY_LICENSES.html $PackageDir

& "$env:JAVA_HOME/bin/jlink.exe" `
    --add-modules java.base,java.desktop,java.logging,java.naming,java.smartcardio,java.sql,jdk.crypto.ec `
    --compress zip-6 `
    --strip-debug `
    --no-header-files `
    --no-man-pages `
    --output "$PackageDir/runtime"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

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
Move-Item -Force $TemporaryArchive "dist/$PackageName.zip"
Remove-Item -Recurse -Force $TemporaryRoot
Write-Output "dist/$PackageName.zip"
