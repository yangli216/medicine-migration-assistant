!macro NSIS_HOOK_POSTINSTALL
  FileOpen $9 "$TEMP\medicine-migration-oracle-hook.txt" w
  FileWrite $9 "oracle-postinstall-started"
  FileClose $9
  DetailPrint "正在登记应用内置的 Oracle Instant Client ODBC 19.31 驱动..."
  SetRegView 64
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\ODBC Drivers" "Oracle in instantclient_19_31_bsoft_migration" "Installed"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "APILevel" "1"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "ConnectFunctions" "YYY"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "CPTimeout" "60"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "Driver" "$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration\sqora32.dll"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "DriverODBCVer" "03.52"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "FileUsage" "0"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "Setup" "$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration\sqoras32.dll"
  WriteRegStr HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration" "SQLLevel" "1"
  Delete "$TEMP\medicine-migration-oracle-hook.txt"
  DetailPrint "Oracle ODBC 驱动登记完成：Oracle in instantclient_19_31_bsoft_migration"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "正在移除应用专用的 Oracle ODBC 驱动登记..."
  SetRegView 64
  DeleteRegValue HKLM "SOFTWARE\ODBC\ODBCINST.INI\ODBC Drivers" "Oracle in instantclient_19_31_bsoft_migration"
  DeleteRegKey HKLM "SOFTWARE\ODBC\ODBCINST.INI\Oracle in instantclient_19_31_bsoft_migration"
!macroend
