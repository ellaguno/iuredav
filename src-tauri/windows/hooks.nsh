; Ganchos del instalador NSIS de IureDav (Tauri los incluye en installer.nsi).
;
; WinFsp: rclone monta la unidad sobre el, y sin el IureDav no puede hacer nada.
; Pedirle al usuario que instale dos programas es pedir demasiado, asi que el MSI
; oficial de WinFsp (descargado por scripts/descargar-winfsp.py) viaja dentro de
; este instalador y se ejecuta en silencio si la maquina no lo tiene ya.
;
; Este instalador se ejecuta por maquina (installMode perMachine), asi que ya va
; elevado y msiexec no pide un segundo permiso de administrador.

; En el nivel superior ${__FILEDIR__} es la carpeta de este fichero (src-tauri/windows).
!define IUREDAV_WINFSP_MSI "${__FILEDIR__}\winfsp.msi"

!macro NSIS_HOOK_POSTINSTALL
  ; Ya instalado: la biblioteca que rclone carga esta en su sitio.
  IfFileExists "$PROGRAMFILES32\WinFsp\bin\winfsp-x64.dll" winfsp_listo
  IfFileExists "$PROGRAMFILES64\WinFsp\bin\winfsp-x64.dll" winfsp_listo

  DetailPrint "Instalando WinFsp (necesario para mostrar la unidad)..."
  SetOutPath "$PLUGINSDIR"
  File "/oname=$PLUGINSDIR\winfsp.msi" "${IUREDAV_WINFSP_MSI}"
  nsExec::ExecToLog 'msiexec.exe /i "$PLUGINSDIR\winfsp.msi" /qn /norestart'
  Pop $0
  ${If} $0 == 0
    DetailPrint "WinFsp instalado."
  ${ElseIf} $0 == 3010
    DetailPrint "WinFsp instalado; Windows pide reiniciar."
    MessageBox MB_OK|MB_ICONINFORMATION "WinFsp se instaló, pero Windows necesita reiniciarse antes de que IureDav pueda montar la unidad.$\r$\n$\r$\nReinicia el equipo cuando te venga bien."
  ${Else}
    DetailPrint "WinFsp no se pudo instalar (código $0)."
    MessageBox MB_OK|MB_ICONEXCLAMATION "No se pudo instalar WinFsp (código $0). IureDav lo necesita para mostrar la unidad.$\r$\n$\r$\nPuedes instalarlo a mano desde https://winfsp.dev/rel/ y IureDav lo detectará solo."
  ${EndIf}

  winfsp_listo:
!macroend
