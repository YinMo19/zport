# Install zport on Windows
# iwr https://raw.githubusercontent.com/YinMo19/zport/refs/heads/master/install.ps1 | iex

param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"
$Repo = "YinMo19/zport"
$Bin = "zport"

$Arch = $env:PROCESSOR_ARCHITECTURE
switch ($Arch) {
    "AMD64" { $GoArch = "x86_64" }
    "ARM64" { $GoArch = "aarch64" }
    default {
        Write-Error "error: unsupported architecture: $Arch"
        exit 1
    }
}

$Target = "${GoArch}-pc-windows-msvc"

if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/${Bin}-${Target}.zip"
} else {
    $Url = "https://github.com/$Repo/releases/download/${Version}/${Bin}-${Target}.zip"
}

Write-Host "Downloading $Bin $Version for $Target..."
$Zip = "$env:TEMP\zport.zip"
Invoke-WebRequest -Uri $Url -OutFile $Zip

$InstallDir = "$env:USERPROFILE\.local\bin"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive -Path $Zip -DestinationPath $InstallDir -Force
Remove-Item $Zip

Write-Host "Installed to $InstallDir\$Bin.exe"

if ($env:PATH -split ';' -notcontains $InstallDir) {
    Write-Host "note: $InstallDir is not in PATH. Add it:"
    Write-Host '  [Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";' + $InstallDir + '", "User")'
}
