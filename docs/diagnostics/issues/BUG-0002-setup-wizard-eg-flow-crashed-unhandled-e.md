---
id: BUG-0002
kind: bug
status: open
area: setup-wizard-eg
fingerprint: e2e:setup-wizard-eg:crash:  - waiting for get_by_label("الدولة")
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 1
---

## setup-wizard-eg flow crashed (unhandled exception):   - waiting for get_by_label("الدولة")

المصدر: e2e flow `setup-wizard-eg`

```
Traceback (most recent call last):
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\run.py", line 276, in main
    rc = flow.run(args.base, shots_dir)
  File "H:\storage\porjcets\invoice-generator\desktop-app\scripts\e2e\flows\setup_wizard_eg.py", line 78, in run
    check(country_select.input_value() == "EG", "the wizard's country step defaults to مصر (EG), not السعودية")
          ~~~~~~~~~~~~~~~~~~~~~~~~~~^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\sync_api\_generated.py", line 19260, in input_value
    self._sync(self._impl_obj.input_value(timeout=to_milliseconds(timeout)))
    ~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_sync_base.py", line 115, in _sync
    return task.result()
           ~~~~~~~~~~~^^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_locator.py", line 501, in input_value
    return await self._frame.input_value(
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ...<3 lines>...
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_frame.py", line 816, in input_value
    return await self._channel.send(
           ^^^^^^^^^^^^^^^^^^^^^^^^^
        "inputValue", self._timeout, locals_to_params(locals())
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_connection.py", line 69, in send
    return await self._connection.wrap_api_call(
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ...<3 lines>...
    )
    ^
  File "C:\Users\abdal\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\playwright\_impl\_connection.py", line 563, in wrap_api_call
    raise rewrite_error(error, f"{parsed_st['apiName']}: {error}") from None
playwright._impl._errors.Error: Locator.input_value: Error: strict mode violation: get_by_label("الدولة") resolved to 2 elements:
    1) <select id="v-1" modelvalue="EG" data-slot="native-select" class="border-input placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 dark:hover:bg-input/50 h-9 min-w-0 appearance-none border bg-transparent px-3 py-2 pe-9 transition-[color,box-shadow] outline-none disabled:pointer-events-none disabled:cursor-not-allowed focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-3 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-des…>…</select> aka get_by_label("الدولة", exact=True)
    2) <button dir="ltr" value="on" role="switch" type="button" data-slot="switch" aria-checked="true" data-state="checked" aria-required="false" class="peer focus-visible:border-ring focus-visible:ring-ring/50 dark:data-[state=unchecked]:bg-input/80 inline-flex shrink-0 items-center rounded-full shadow-xs transition-all outline-none focus-visible:ring-3 disabled:cursor-not-allowed disabled:opacity-50 mt-0.5 h-[1.15rem] w-8 border border-border-control data-[state=checked]:border-primary data-[state=checked]…>…</button> aka get_by_role("switch", name="الأسعار المعروضة شاملة الضريبة الوضع الافتراضي للتجزئة في هذه الدولة — يمكن تغيي")

Call log:
  - waiting for get_by_label("الدولة")


```
