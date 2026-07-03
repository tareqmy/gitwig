$ErrorActionPreference = 'Stop'

$toolsDir   = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$packageId  = 'gitwig'
$url64      = 'https://github.com/tareqmy/gitwig/releases/download/v2.3.17/gitwig-v2.3.17-x86_64-pc-windows-msvc.zip'
$checksum64 = 'WINDOWS_ZIP_SHA256' # Automatically updated by CI on release

$packageArgs = @{
  packageName   = $packageId
  unzipLocation = $toolsDir
  url64bit      = $url64
  checksum64    = $checksum64
  checksumType64= 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Creates command shim in Choco bin directory pointing to extracted gitwig.exe
$exePath = Join-Path $toolsDir "gitwig.exe"
Install-BinFile -Name "gitwig" -Path $exePath
