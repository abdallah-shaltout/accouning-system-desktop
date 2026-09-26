---
id: BUG-0004
kind: bug
status: open
area: purchases
fingerprint: e2e:purchases:crash:  - waiting for locator("table tbody tr").first.locator("input[type=number]").first
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 2
---

## purchases flow crashed (unhandled exception):   - waiting for locator("table tbody tr").first.locator("input[type=number]").first

المصدر: e2e flow `purchases`

```
Traceback (most recent call last):
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\run.py", line 276, in main
    rc = flow.run(args.base, shots_dir)
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\flows\purchases.py", line 67, in run
    qty_input.click()
    ~~~~~~~~~~~~~~~^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\sync_api\_generated.py", line 17672, in click
    self._sync(
    ~~~~~~~~~~^
        self._impl_obj.click(
        ^^^^^^^^^^^^^^^^^^^^^
    ...<10 lines>...
        )
        ^
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_sync_base.py", line 115, in _sync
    return task.result()
           ~~~~~~~~~~~^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_locator.py", line 163, in click
    return await self._frame._click(self._selector, strict=True, **params)
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_frame.py", line 591, in _click
    await self._channel.send("click", self._timeout, locals_to_params(locals()))
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_connection.py", line 69, in send
    return await self._connection.wrap_api_call(
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ...<3 lines>...
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_connection.py", line 563, in wrap_api_call
    raise rewrite_error(error, f"{parsed_st['apiName']}: {error}") from None
playwright._impl._errors.TimeoutError: Locator.click: Timeout 30000ms exceeded.
Call log:
  - waiting for locator("table tbody tr").first.locator("input[type=number]").first


```
