!macro KillCoreviewProcesses
  nsExec::Exec 'taskkill /F /T /IM "coreview-sensors.exe"'
  nsExec::Exec 'taskkill /F /T /IM "PresentMon.exe"'
  nsExec::Exec 'taskkill /F /T /IM "Fairstack Coreview.exe"'
  Sleep 1000
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro KillCoreviewProcesses
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro KillCoreviewProcesses
!macroend
