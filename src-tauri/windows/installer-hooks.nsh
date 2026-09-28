; Installer hooks for the bundled MariaDB server (phase-a2 A2-2, P2-43/C-21/C-26).
;
; PREUNINSTALL: stop a running managed server *before* files under $INSTDIR are removed, so
; nothing has server-bin\...\mariadbd.exe open when NSIS tries to delete it. Then remove only
; the installed server-bin *copy* under %ProgramData% — never the data directory, server.json or
; pre-upgrade snapshots, which hold the customer's actual business data and must survive an
; uninstall (and a "manual upgrade" that uninstalls-then-reinstalls).
;
; POSTUNINSTALL: tell the user, in Arabic, that their data was not deleted and where it lives.
;
; There is no PREINSTALL hook: nothing the installer writes during install is ever locked by a
; running server (P2-45 — the installer only ever touches $INSTDIR\mariadb, the pristine payload
; copy; the running server always executes from %ProgramData%\...\database\server-bin\current,
; a separate copy made at provision/upgrade time).

!macro NSIS_HOOK_PREUNINSTALL
  ; Only installs from phase A2 onward have $INSTDIR\mariadb\bin\mariadbd.exe at all — an older
  ; install has no such folder, and this exe must never be launched with an argument it doesn't
  ; understand, so check for it first.
  IfFileExists "$INSTDIR\mariadb\bin\mariadbd.exe" 0 skip_db_shutdown
    nsExec::Exec '"$INSTDIR\${MAINBINARYNAME}.exe" --db-shutdown'
  skip_db_shutdown:

  ReadEnvStr $0 "ProgramData"
  ; Never touch database\data, server.json or pre-upgrade — only the installed server binaries
  ; copy, which the next install (or the running app, on its next provision/upgrade check)
  ; recreates from the payload it ships.
  RMDir /r "$0\${BUNDLEID}\database\server-bin"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ReadEnvStr $0 "ProgramData"
    IfFileExists "$0\${BUNDLEID}\database\server.json" 0 skip_notice
      MessageBox MB_OK|MB_ICONINFORMATION "تم إلغاء تثبيت ${PRODUCTNAME}. بيانات نشاطك التجاري محفوظة ولم تُحذف في المجلد: $0\${BUNDLEID}\database — عند تثبيت البرنامج مرة أخرى على هذا الجهاز سيعمل على نفس البيانات تلقائياً. لا تحذف هذا المجلد إلا إذا كانت لديك نسخة احتياطية." /SD IDOK
    skip_notice:
  ${EndIf}
!macroend
