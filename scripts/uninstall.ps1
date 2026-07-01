# Gitwig Uninstaller for Windows
# Usage: .\uninstall.ps1

$ErrorActionPreference = "Stop"

function Write-Info ($msg) {
    Write-Host -ForegroundColor Cyan "[info] $msg"
}
function Write-Success ($msg) {
    Write-Host -ForegroundColor Green "[success] $msg"
}

$install_dir = Join-Path $env:USERPROFILE ".gitwig\bin"
$exe_path = Join-Path $install_dir "gitwig.exe"

if (Test-Path $exe_path) {
    Write-Info "Removing gitwig.exe..."
    Remove-Item -Path $exe_path -Force
}

# Remove directory if empty
if (Test-Path $install_dir) {
    $files = Get-ChildItem -Path $install_dir
    if ($files.Count -eq 0) {
        Write-Info "Removing empty bin directory..."
        Remove-Item -Path $install_dir -Force
    }
}

# Remove from Path
Write-Info "Removing $install_dir from User PATH..."
$path_var = [Environment]::GetEnvironmentVariable("Path", "User")
if ($path_var) {
    $parts = $path_var -split ";" | Where-Object { $_ -ne "" -and $_ -ne $install_dir }
    $new_path = $parts -join ";"
    [Environment]::SetEnvironmentVariable("Path", $new_path, "User")
}

Write-Success "Gitwig has been uninstalled."
