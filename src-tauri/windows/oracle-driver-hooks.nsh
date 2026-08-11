!macro NSIS_HOOK_POSTINSTALL
  FileOpen $9 "$TEMP\medicine-migration-oracle-hook.txt" w
  FileWrite $9 "oracle-postinstall-started"
  FileClose $9
  DetailPrint "正在登记应用内置的 Oracle Instant Client ODBC 19.31 驱动..."
  SetOutPath "$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration"
  nsExec::ExecToStack '"$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration\odbc_install.exe"'
  Pop $0
  Pop $1
  SetOutPath "$INSTDIR"
  StrCmp $0 "0" oracle_driver_install_done
  DetailPrint "Oracle ODBC 驱动登记失败，退出码：$0，输出：$1"
  MessageBox MB_ICONSTOP|MB_OK "应用内置的 Oracle ODBC 驱动登记失败（退出码 $0）。请确认使用管理员权限安装。"
  Abort
oracle_driver_install_done:
  Delete "$TEMP\medicine-migration-oracle-hook.txt"
  DetailPrint "Oracle ODBC 驱动登记完成：Oracle in instantclient_19_31_bsoft_migration"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  IfFileExists "$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration\odbc_uninstall.exe" 0 oracle_driver_uninstall_done
  DetailPrint "正在移除应用专用的 Oracle ODBC 驱动登记..."
  SetOutPath "$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration"
  nsExec::ExecToLog '"$INSTDIR\resources\oracle\instantclient_19_31_bsoft_migration\odbc_uninstall.exe"'
  SetOutPath "$INSTDIR"
oracle_driver_uninstall_done:
!macroend
