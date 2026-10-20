param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64"
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$target = if ($Arch -eq "arm64") {
    "aarch64-pc-windows-msvc"
} else {
    "x86_64-pc-windows-msvc"
}
# cargo-packager's WiX 3 backend cannot emit ARM64 MSI packages.
$formats = if ($Arch -eq "arm64") {
    "nsis"
} else {
    "nsis,wix"
}
$version = (Select-String -Path "Cargo.toml" -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
$packagerVersion = (Select-String -Path "Packager.toml" -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
if ($version -ne $packagerVersion) {
    throw "Version mismatch: Cargo.toml=$version, Packager.toml=$packagerVersion"
}

if (Test-Path "dist") {
    Get-ChildItem -LiteralPath "dist" -File |
        Where-Object { $_.Name -like "rshell-*" -or $_.Name -like "rshell_*" } |
        Remove-Item -Force
}

if (-not (Get-Command cargo-packager -ErrorAction SilentlyContinue)) {
    cargo install cargo-packager --version 0.11.8 --locked
}

rustup target add $target
cargo build --release --target $target --bin rshell
New-Item -ItemType Directory -Force -Path "dist" | Out-Null
Copy-Item -Force "target/$target/release/rshell.exe" "dist/rshell.exe"
try {
    cargo packager --release --target $target --formats $formats --out-dir dist
}
finally {
    Remove-Item -LiteralPath "dist/rshell.exe" -Force -ErrorAction SilentlyContinue
}

$installer = Get-ChildItem "dist" -Filter "rshell_*_*-setup.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $installer) {
    throw "cargo-packager did not produce an NSIS installer"
}
$installerName = "rshell-$version-windows-$Arch-setup.exe"
Move-Item -Force $installer.FullName (Join-Path "dist" $installerName)

if ($Arch -eq "x64") {
    $msi = Get-ChildItem "dist" -Filter "rshell_*.msi" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $msi) {
        throw "cargo-packager did not produce an MSI installer"
    }
    $msiName = "rshell-$version-windows-$Arch.msi"
    Move-Item -Force $msi.FullName (Join-Path "dist" $msiName)
}
else {
    Write-Host "Skipping MSI: WiX 3 does not support ARM64 packages."
}

$portableExeName = "rshell-$version-windows-$Arch.exe"
Copy-Item -Force `
    "target/$target/release/rshell.exe" `
    (Join-Path "dist" $portableExeName)

$portableName = "rshell-$version-windows-$Arch.zip"
Compress-Archive -Force `
    -Path "target/$target/release/rshell.exe" `
    -DestinationPath (Join-Path "dist" $portableName)

if ($Arch -eq "x64") {
    $staticExeName = "rshell-$version-windows-static-$Arch.exe"
    $previousRustFlags = $env:RUSTFLAGS
    $previousTargetDir = $env:CARGO_TARGET_DIR
    try {
        $staticFlag = "-C target-feature=+crt-static"
        $env:RUSTFLAGS = if ($previousRustFlags) {
            "$previousRustFlags $staticFlag"
        } else {
            $staticFlag
        }
        $env:CARGO_TARGET_DIR = "target/static"
        cargo build --release --target $target --bin rshell
    }
    finally {
        if ($null -eq $previousRustFlags) {
            Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
        }
        else {
            $env:RUSTFLAGS = $previousRustFlags
        }
        if ($null -eq $previousTargetDir) {
            Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
        }
        else {
            $env:CARGO_TARGET_DIR = $previousTargetDir
        }
    }
    Copy-Item -Force `
        "target/static/$target/release/rshell.exe" `
        (Join-Path "dist" $staticExeName)
}

Write-Host "Created: dist/$portableExeName"
Write-Host "Created: dist/$installerName"
if ($Arch -eq "x64") {
    Write-Host "Created: dist/$msiName"
    Write-Host "Created: dist/$staticExeName"
}
Write-Host "Created: dist/$portableName"
