$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$oracleDirectory = Join-Path $repoRoot "src-tauri/resources/oracle/instantclient_19_31_bsoft_migration"
$driverName = "Oracle in instantclient_19_31_bsoft_migration"
$driverRegistryPath = "HKLM:\SOFTWARE\ODBC\ODBCINST.INI\$driverName"

function Invoke-OracleTool([string]$ToolName, [int]$TimeoutSeconds = 45) {
  $tool = Join-Path $oracleDirectory $ToolName
  if (-not (Test-Path $tool -PathType Leaf)) {
    throw "Oracle 驱动工具不存在：$tool"
  }
  $stdout = Join-Path $env:RUNNER_TEMP "$ToolName.stdout.log"
  $stderr = Join-Path $env:RUNNER_TEMP "$ToolName.stderr.log"
  Remove-Item $stdout, $stderr -Force -ErrorAction SilentlyContinue
  Write-Host "执行 Oracle 驱动工具：$ToolName"
  $process = Start-Process -FilePath $tool -WorkingDirectory $oracleDirectory `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
  if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    throw "Oracle 驱动工具执行超时：$ToolName"
  }
  $output = @(
    (Get-Content $stdout -Raw -ErrorAction SilentlyContinue),
    (Get-Content $stderr -Raw -ErrorAction SilentlyContinue)
  ) -join "`n"
  Write-Host $output.Trim()
  if ($process.ExitCode -ne 0) {
    throw "Oracle 驱动工具执行失败：$ToolName，退出码 $($process.ExitCode)"
  }
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
$isAdministrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "Windows 管理员权限：$isAdministrator"
if (-not $isAdministrator) {
  throw "Oracle ODBC 登记验收需要管理员权限"
}

if (Test-Path $driverRegistryPath) {
  Invoke-OracleTool "odbc_uninstall.exe"
}
Invoke-OracleTool "odbc_install.exe"
if (-not (Test-Path $driverRegistryPath)) {
  throw "Oracle 官方登记工具执行后未发现 64 位 ODBC 驱动：$driverName"
}
$registeredDriver = [string](Get-ItemProperty $driverRegistryPath).Driver
$expectedDriver = Join-Path $oracleDirectory "sqora32.dll"
if (-not [System.IO.Path]::GetFullPath($registeredDriver).Equals(
    [System.IO.Path]::GetFullPath($expectedDriver),
    [System.StringComparison]::OrdinalIgnoreCase
  )) {
  throw "Oracle 官方登记工具写入了错误路径：$registeredDriver"
}
Invoke-OracleTool "odbc_uninstall.exe"
if (Test-Path $driverRegistryPath) {
  throw "Oracle 官方卸载工具执行后仍保留驱动登记：$driverName"
}
Write-Host "Oracle 官方 ODBC 登记与卸载工具验收通过"
