# Gitwig Installer for Windows
# Supported Platforms: Windows (x86_64)
# Usage: irm https://raw.githubusercontent.com/tareqmy/gitwig/master/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

$repo_owner = "tareqmy"
$repo_name = "gitwig"
$github_raw_url = "https://raw.githubusercontent.com/$repo_owner/$repo_name/master"
$github_api_url = "https://api.github.com/repos/$repo_owner/$repo_name"
$github_releases_url = "https://github.com/$repo_owner/$repo_name/releases"

# Colors for output
function Write-Info ($msg) {
    Write-Host -ForegroundColor Cyan "[info] $msg"
}
function Write-Success ($msg) {
    Write-Host -ForegroundColor Green "[success] $msg"
}
function Write-WarningMsg ($msg) {
    Write-Host -ForegroundColor Yellow "[warn] $msg"
}
function Write-ErrorMsg ($msg) {
    Write-Host -ForegroundColor Red "[error] $msg"
    exit 1
}

# 1. Detect Architecture
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq "AMD64") {
    $target = "x86_64-pc-windows-msvc"
} else {
    Write-ErrorMsg "Unsupported Windows architecture: $arch. Gitwig currently only supports x86_64 Windows."
}

# 2. Resolve version
Write-Info "Querying latest version..."
$version = ""
if (Test-Path ".version") {
    $version = (Get-Content ".version").Trim()
    Write-Info "Found local .version file: $version"
} else {
    $version_url = "$github_raw_url/.version"
    $headers = @{}
    if ($env:GITHUB_TOKEN) {
        $headers["Authorization"] = "token $env:GITHUB_TOKEN"
    }
    
    try {
        $version = (Invoke-RestMethod -Uri $version_url -Headers $headers).Trim()
    } catch {
        # Fallback to GitHub API
        try {
            $api_url = "$github_api_url/releases/latest"
            $release = Invoke-RestMethod -Uri $api_url -Headers $headers
            $version = $release.tag_name
        } catch {
            Write-ErrorMsg "Could not resolve latest version. Please check your internet connection or GITHUB_TOKEN."
        }
    }
}

if (-not $version.StartsWith("v")) {
    $version = "v$version"
}
Write-Info "Using version: $version"

# 3. Choose installation directory
$install_dir = Join-Path $env:USERPROFILE ".gitwig\bin"
if (-not (Test-Path $install_dir)) {
    New-Item -ItemType Directory -Path $install_dir -Force | Out-Null
}
Write-Info "Selected installation directory: $install_dir"

# 4. Download and Extract
$download_url = "$github_releases_url/download/$version/gitwig-$version-$target.zip"
$tmp_dir = Join-Path $env:TEMP "gitwig-install-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Path $tmp_dir -Force | Out-Null

$zip_path = Join-Path $tmp_dir "gitwig.zip"

Write-Info "Downloading Gitwig from $download_url..."
try {
    # If GITHUB_TOKEN is set, resolve asset and download via API
    if ($env:GITHUB_TOKEN) {
        Write-Info "Using GITHUB_TOKEN for download..."
        $headers = @{"Authorization" = "token $env:GITHUB_TOKEN"}
        $tag_api_url = "$github_api_url/releases/tags/$version"
        $release_json = Invoke-RestMethod -Uri $tag_api_url -Headers $headers
        $asset_name = "gitwig-$version-$target.zip"
        
        $asset = $release_json.assets | Where-Object { $_.name -eq $asset_name }
        if (-not $asset) {
            $alt_name = "gitwig-$target.zip"
            $asset = $release_json.assets | Where-Object { $_.name -eq $alt_name }
        }
        
        if ($asset) {
            $download_url = $asset.url
            $headers["Accept"] = "application/octet-stream"
            Invoke-WebRequest -Uri $download_url -Headers $headers -OutFile $zip_path
        } else {
            Write-ErrorMsg "Failed to find release asset for $target"
        }
    } else {
        Invoke-WebRequest -Uri $download_url -OutFile $zip_path
    }
} catch {
    Write-ErrorMsg "Download failed. Please check your network or GITHUB_TOKEN. Details: $_"
}

Write-Info "Extracting archive..."
try {
    Expand-Archive -Path $zip_path -DestinationPath $tmp_dir -Force
} catch {
    Write-ErrorMsg "Failed to extract zip archive. Details: $_"
}

# Find gitwig.exe and move it
$exe_path = Join-Path $tmp_dir "gitwig.exe"
if (-not (Test-Path $exe_path)) {
    # Try looking inside subfolders in case of double packaging
    $exe_path = Get-ChildItem -Path $tmp_dir -Filter "gitwig.exe" -Recurse | Select-Object -First 1
}

if (-not $exe_path) {
    Write-ErrorMsg "Could not find gitwig.exe in the extracted archive."
}

$dest_path = Join-Path $install_dir "gitwig.exe"
Write-Info "Installing gitwig.exe to $dest_path..."
Move-Item -Path $exe_path -Destination $dest_path -Force

# Create gtg.exe shortcut copy
$gtg_dest_path = Join-Path $install_dir "gtg.exe"
Write-Info "Creating shortcut gtg.exe..."
Copy-Item -Path $dest_path -Destination $gtg_dest_path -Force

# Clean up
Remove-Item -Path $tmp_dir -Recurse -Force

# 5. Add to Path
Write-Info "Adding $install_dir to User PATH..."
$path_var = [Environment]::GetEnvironmentVariable("Path", "User")
$path_parts = $path_var -split ";" | Where-Object { $_ -ne "" }
if ($path_parts -notcontains $install_dir) {
    [Environment]::SetEnvironmentVariable("Path", "$path_var;$install_dir", "User")
    $env:Path += ";$install_dir"
    Write-Success "Added to User PATH successfully."
} else {
    Write-Info "Directory already in PATH."
}

Write-Success "Gitwig has been successfully installed!"
Write-Host ""
Write-Host -ForegroundColor Green "To start using Gitwig, please restart your terminal / PowerShell session and run:"
Write-Host -ForegroundColor Green "    gitwig (or shortcut: gtg)"

