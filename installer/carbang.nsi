; Установщик CarBang (NSIS 3, Modern UI 2). Сборка: makensis installer/carbang.nsi
Unicode true
!include "MUI2.nsh"

!define APPNAME "CarBang"
!define VERSION "0.2.0"
!define PUBLISHER "CarBang"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"

Name "${APPNAME}"
OutFile "..\dist\CarBang-Setup-${VERSION}.exe"
InstallDir "$PROGRAMFILES64\${APPNAME}"
InstallDirRegKey HKLM "Software\${APPNAME}" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
BrandingText "${APPNAME} ${VERSION}"

!define MUI_ICON "carbang.ico"
!define MUI_UNICON "carbang.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\carbang.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Запустить CarBang"
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\README.txt"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Открыть описание и управление"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\dist\stage\LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"

Section "CarBang (обязательно)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "..\dist\stage\carbang.exe"
  File "..\dist\stage\README.txt"
  File "..\dist\stage\LICENSE.txt"
  SetOutPath "$INSTDIR\licenses"
  File /r "..\dist\stage\licenses\*.*"
  SetOutPath "$INSTDIR\assets"
  File /r "..\dist\stage\assets\*.*"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\${APPNAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTKEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\carbang.exe"
  WriteRegStr HKLM "${UNINSTKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoRepair" 1
  CreateDirectory "$SMPROGRAMS\${APPNAME}"
  CreateShortcut "$SMPROGRAMS\${APPNAME}\CarBang.lnk" "$INSTDIR\carbang.exe"
  CreateShortcut "$SMPROGRAMS\${APPNAME}\Удалить CarBang.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Ярлык на рабочем столе" SecDesktop
  CreateShortcut "$DESKTOP\CarBang.lnk" "$INSTDIR\carbang.exe"
SectionEnd

Section "Правило брандмауэра для сетевой игры" SecFirewall
  nsExec::Exec 'netsh advfirewall firewall add rule name="CarBang" dir=in action=allow program="$INSTDIR\carbang.exe" enable=yes'
SectionEnd

Section "Uninstall"
  nsExec::Exec 'netsh advfirewall firewall delete rule name="CarBang"'
  Delete "$DESKTOP\CarBang.lnk"
  RMDir /r "$SMPROGRAMS\${APPNAME}"
  RMDir /r "$INSTDIR\licenses"
  RMDir /r "$INSTDIR\assets"
  Delete "$INSTDIR\carbang.exe"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINSTKEY}"
  DeleteRegKey HKLM "Software\${APPNAME}"
SectionEnd
