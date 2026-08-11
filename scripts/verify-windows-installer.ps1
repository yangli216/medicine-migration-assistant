param(
  [string]$InstallerPath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $InstallerPath) {
  $candidate = Get-ChildItem "$repoRoot/src-tauri/target/release/bundle/nsis/*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if (-not $candidate) {
    throw "没有找到 NSIS 安装包"
  }
  $InstallerPath = $candidate.FullName
}
$InstallerPath = (Resolve-Path $InstallerPath).Path

function Get-PeMachine([string]$Path) {
  $stream = [System.IO.File]::OpenRead($Path)
  try {
    $reader = [System.IO.BinaryReader]::new($stream)
    $stream.Position = 0x3c
    $peOffset = $reader.ReadInt32()
    $stream.Position = $peOffset + 4
    return $reader.ReadUInt16()
  } finally {
    $stream.Dispose()
  }
}

$installRoot = Join-Path $env:RUNNER_TEMP "medicine-migration-install-acceptance"
$hookMarker = Join-Path $env:RUNNER_TEMP "medicine-migration-oracle-hook.txt"
if (Test-Path $installRoot) {
  Remove-Item $installRoot -Recurse -Force
}
Remove-Item $hookMarker -Force -ErrorAction SilentlyContinue
New-Item $installRoot -ItemType Directory | Out-Null

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
$isAdministrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "开始静默安装；Windows 管理员权限：$isAdministrator"
if (-not $isAdministrator) {
  throw "perMachine 安装与 Oracle ODBC 登记验收需要管理员权限"
}
$installerProcess = Start-Process -FilePath $InstallerPath -ArgumentList "/S", "/D=$installRoot" -PassThru
if (-not $installerProcess.WaitForExit(180000)) {
  $phase = if (Test-Path $hookMarker) {
    "已进入 Oracle ODBC 登记钩子"
  } elseif (Test-Path $installRoot) {
    "尚未进入 Oracle ODBC 登记钩子，安装目录已创建"
  } else {
    "尚未创建安装目录"
  }
  Stop-Process -Id $installerProcess.Id -Force -ErrorAction SilentlyContinue
  throw "NSIS 静默安装超过 180 秒（$phase）"
}
if ($installerProcess.ExitCode -ne 0) {
  throw "NSIS 静默安装失败，退出码 $($installerProcess.ExitCode)"
}

$application = Get-ChildItem $installRoot -Filter "*.exe" -Recurse |
  Where-Object { $_.Name -notmatch "(?i)uninstall" } |
  Select-Object -First 1
if (-not $application) {
  throw "安装完成后未找到应用程序 EXE"
}

$machine = Get-PeMachine $application.FullName
if ($machine -ne 0x8664) {
  throw ("应用程序不是 Windows x64 PE，Machine=0x{0:X4}" -f $machine)
}

$oracleDriverName = "Oracle in instantclient_19_31_bsoft_migration"
$oracleDirectory = Join-Path $installRoot "resources/oracle/instantclient_19_31_bsoft_migration"
$oracleRequiredFiles = @(
  "BASIC_LICENSE",
  "ODBC_LICENSE",
  "oci.dll",
  "oraociei19.dll",
  "sqora32.dll",
  "sqoras32.dll",
  "odbc_install.exe",
  "odbc_uninstall.exe",
  "BUNDLE_MANIFEST.json"
)
foreach ($fileName in $oracleRequiredFiles) {
  $filePath = Join-Path $oracleDirectory $fileName
  if (-not (Test-Path $filePath -PathType Leaf)) {
    throw "安装后的 Oracle 内置驱动缺少文件：$fileName"
  }
}

$oracleRegistryPath = "HKLM:\SOFTWARE\ODBC\ODBCINST.INI\$oracleDriverName"
if (-not (Test-Path $oracleRegistryPath)) {
  throw "Oracle 内置 ODBC 驱动未登记到 64 位系统驱动清单：$oracleDriverName"
}
$oracleRegistry = Get-ItemProperty $oracleRegistryPath
$registeredOracleDriver = ([string]$oracleRegistry.Driver).Trim([char]0)
$expectedOracleDriver = Join-Path $oracleDirectory "sqora32.dll"
if (-not $registeredOracleDriver -or
    -not [System.IO.Path]::GetFullPath($registeredOracleDriver).Equals(
      [System.IO.Path]::GetFullPath($expectedOracleDriver),
      [System.StringComparison]::OrdinalIgnoreCase
    )) {
  throw "Oracle ODBC 驱动登记路径错误：$registeredOracleDriver"
}

$appProcess = Start-Process -FilePath $application.FullName -PassThru
Start-Sleep -Seconds 8
$launched = -not $appProcess.HasExited
if (-not $launched -and $appProcess.ExitCode -ne 0) {
  throw "安装后的桌面应用启动失败，退出码 $($appProcess.ExitCode)"
}
if ($launched) {
  Stop-Process -Id $appProcess.Id -Force
  $appProcess.WaitForExit()
}

$signature = Get-AuthenticodeSignature $application.FullName
$reportDirectory = Join-Path $repoRoot "artifacts"
New-Item $reportDirectory -ItemType Directory -Force | Out-Null
$report = [ordered]@{
  installer = $InstallerPath
  installedExecutable = $application.FullName
  architecture = "x86_64"
  silentInstallExitCode = $installerProcess.ExitCode
  applicationLaunchObserved = $launched
  bundledOracleVersion = "19.31.0.0.0"
  bundledOracleDriver = $oracleDriverName
  oracleDriverRegistered = $true
  oracleDriverPath = $registeredOracleDriver
  authenticodeStatus = $signature.Status.ToString()
  verifiedAt = [DateTimeOffset]::UtcNow.ToString("O")
}
$report | ConvertTo-Json | Set-Content (Join-Path $reportDirectory "windows-install-acceptance.json") -Encoding UTF8
$report | ConvertTo-Json
