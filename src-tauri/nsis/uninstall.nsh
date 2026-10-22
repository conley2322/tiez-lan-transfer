# Hooks for the NSIS installer.
#
# This fork is a file transfer tool: the clipboard feature it came from is not
# built, so nothing here touches the clipboard policy keys any more — restoring
# them would be an uninstall step for a switch this app never moved.
# What is left is everything TieZ really does to this machine: the process, the
# autostart value, the right-click submenu, and its own files.

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Stopping TieZ before uninstall..."

  # 应用关闭主窗口时会缩到托盘，卸载器不能依赖普通的关闭请求。
  # 这里在开始删除文件前强制结束已安装的进程，避免 exe 被占用。
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /T /IM TieZ.exe'
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /T /IM tiez-app.exe'
  Sleep 1200

  # 开机自启动（程序自己写的 HKCU Run 值）。
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "TieZ"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tie-z"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tiez-app"

  # 右键「发送给」子菜单：程序名和路径都写在里面，卸载后留着就是死项。
  DeleteRegKey HKCU "Software\Classes\*\shell\TieZSend"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\TieZSend"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  # 资源管理器缓存了右键菜单项，注册表已经删掉的子菜单要等它重读一次。
  DetailPrint "Restarting Explorer to drop the right-click menu..."
  nsExec::Exec '"powershell.exe" -NoProfile -WindowStyle Hidden -Command "Stop-Process -Name explorer -Force; Start-Process explorer"'
  DetailPrint "Explorer restarted."

  # 如果卸载时程序刚被关闭，或者 uninstall.exe 还没完全退出，
  # NSIS 可能暂时删不掉安装目录。这里额外拉起一个后台清理助手，
  # 等卸载器退出后再尝试删除残留目录和文件。
  DetailPrint "Scheduling leftover install directory cleanup..."
  nsExec::Exec '"$SYSDIR\cmd.exe" /C start "" /MIN powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command "Start-Sleep -Seconds 3; Stop-Process -Name TieZ,tiez-app -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath ''$INSTDIR'' -Force -Recurse -ErrorAction SilentlyContinue"'
!macroend
