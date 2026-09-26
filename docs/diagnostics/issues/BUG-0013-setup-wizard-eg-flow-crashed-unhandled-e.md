---
id: BUG-0013
kind: bug
status: open
area: setup-wizard-eg
fingerprint: e2e:setup-wizard-eg:crash:  - navigating to "http://localhost:1420/#/welcome", waiting until "load"
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 1
---

## setup-wizard-eg flow crashed (unhandled exception):   - navigating to "http://localhost:1420/#/welcome", waiting until "load"

المصدر: e2e flow `setup-wizard-eg`

```
Traceback (most recent call last):
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\run.py", line 281, in main
    rc = flow.run(args.base, shots_dir)
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\flows\setup_wizard_eg.py", line 52, in run
    clear_snapshot(page, base)
    ~~~~~~~~~~~~~~^^^^^^^^^^^^
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\flows\setup_wizard_eg.py", line 33, in clear_snapshot
    page.goto(f"{base}/welcome")
    ~~~~~~~~~^^^^^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\sync_api\_generated.py", line 9770, in goto
    self._sync(
    ~~~~~~~~~~^
        self._impl_obj.goto(
        ^^^^^^^^^^^^^^^^^^^^
    ...<4 lines>...
        )
        ^
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_sync_base.py", line 115, in _sync
    return task.result()
           ~~~~~~~~~~~^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_page.py", line 560, in goto
    return await self._main_frame.goto(**locals_to_params(locals()))
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_frame.py", line 156, in goto
    await self._channel.send(
        "goto", self._navigation_timeout, locals_to_params(locals())
    )
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_connection.py", line 69, in send
    return await self._connection.wrap_api_call(
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ...<3 lines>...
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_connection.py", line 563, in wrap_api_call
    raise rewrite_error(error, f"{parsed_st['apiName']}: {error}") from None
playwright._impl._errors.Error: Page.goto: net::ERR_CONNECTION_REFUSED at http://localhost:1420/#/welcome
Call log:
  - navigating to "http://localhost:1420/#/welcome", waiting until "load"


```
