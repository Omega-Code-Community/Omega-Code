!define PRODUCT_NAME "OmegaCode"

OutFile "${PRODUCT_NAME}_${VERSION}_windows_${ARCH}.exe"

InstallDir "$PROGRAMFILES64\OmegaCode"

RequestExecutionLevel admin

Page directory
Page instfiles

Section

SetOutPath "$INSTDIR"

File /r "build\release_bundle\*"

WriteUninstaller "$INSTDIR\uninstall.exe"

SectionEnd

Section "Uninstall"

Delete "$INSTDIR\uninstall.exe"

RMDir /r "$INSTDIR"

SectionEnd