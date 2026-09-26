---
id: BUG-0010
kind: bug
status: open
area: setup-wizard-eg
fingerprint: e2e:setup-wizard-eg:crash:============================================================
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 1
---

## setup-wizard-eg flow crashed (unhandled exception): ============================================================

المصدر: e2e flow `setup-wizard-eg`

```
Traceback (most recent call last):
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\run.py", line 281, in main
    rc = flow.run(args.base, shots_dir)
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\flows\setup_wizard_eg.py", line 109, in run
    page.wait_for_url(lambda u: "/login" in u, timeout=15000)
    ~~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\sync_api\_generated.py", line 9924, in wait_for_url
    self._sync(
    ~~~~~~~~~~^
        self._impl_obj.wait_for_url(
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ...<3 lines>...
        )
        ^
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_sync_base.py", line 115, in _sync
    return task.result()
           ~~~~~~~~~~~^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_page.py", line 588, in wait_for_url
    return await self._main_frame.wait_for_url(**locals_to_params(locals()))
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_frame.py", line 288, in wait_for_url
    async with self.expect_navigation(
               ~~~~~~~~~~~~~~~~~~~~~~^
        url=url, waitUntil=waitUntil, timeout=timeout
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ):
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_event_context_manager.py", line 33, in __aexit__
    await self._future
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_frame.py", line 264, in continuation
    event = await waiter.result()
            ^^^^^^^^^^^^^^^^^^^^^
playwright._impl._errors.TimeoutError: Timeout 15000ms exceeded.
=========================== logs ===========================
waiting for navigation to "<function ImplToApiMapping.wrap_handler.<locals>.wrapper_func at 0x00000228B0A50250>" until 'load'
============================================================

```
