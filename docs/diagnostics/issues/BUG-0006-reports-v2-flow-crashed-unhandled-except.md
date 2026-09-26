---
id: BUG-0006
kind: bug
status: open
area: reports-v2
fingerprint: e2e:reports-v2:crash:============================================================
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 1
---

## reports-v2 flow crashed (unhandled exception): ============================================================

المصدر: e2e flow `reports-v2`

```
Traceback (most recent call last):
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\run.py", line 276, in main
    rc = flow.run(args.base, shots_dir)
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\flows\reports_v2.py", line 85, in run
    with page.expect_download(timeout=15000) as dl_info:
         ~~~~~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_sync_base.py", line 85, in __exit__
    self._event.value
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_sync_base.py", line 59, in value
    raise exception
playwright._impl._errors.TimeoutError: Timeout 15000ms exceeded while waiting for event "download"
=========================== logs ===========================
waiting for event "download"
============================================================

```
