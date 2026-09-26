# 03 — Chart of Accounts v2 (شجرة الحسابات)

**Changes from v1:**

- The tree has **header accounts** (non-postable) and **leaf accounts** (postable).
- Accounts carry a _subtype_ for statement classification.
- Posting finds accounts by **system role**, never by code.
- Onboarding offers three templates, plus country and business-type add-ons.

## 1. Account shape (additions to v1)

```ts
interface Account {
    id: string;
    code: string;
    name: string;
    nameEn?: string;
    parentId: string | null; // tree
    isGroup: boolean; // header: can't be posted to; its balance rolls up from children
    kind: "ASSET" | "LIABILITY" | "EQUITY" | "REVENUE" | "EXPENSE";
    subtype: AccountSubtype; // see §3
    normalSide: "DEBIT" | "CREDIT"; // default from kind; contra accounts flip it
    systemRole?: SystemRole; // at most one account per role (branch-scoped roles excepted)
    currency?: string; // cash/bank in a foreign currency; null = base currency
    branchId?: string; // branch cash drawers
    requiresParty?: boolean; // AR/AP control accounts
    allowManual: boolean; // false for inventory, VAT in/out, and virtual accounts
    requiresCostCenter?: boolean; // optional per expense account
    active: boolean;
    canDelete: boolean; // system-role accounts can't be deleted, only renamed/renumbered
}
```

## 2. System roles

```text
cash* bank* cardClearing walletClearing receivable inventory inventoryInTransit vatInput
payable vatOutput vatPayable customerAdvances capital ownerCurrent drawings retainedEarnings
currentEarnings(virtual) openingBalanceEquity sales serviceRevenue salesReturns otherIncome
fxGain cashOver purchaseDiscounts cogs inventoryVariance inventoryWriteOff freightIn
cardFees bankFees fxLoss cashShort badDebt depreciation zakat
```

`*` = several accounts can hold this role, e.g. one cash drawer per branch or several banks. Payment
methods and branches point at the specific account.

- **Lookup:** `accountFor(role, { branchId?, currency? })` returns the account, or throws an Arabic
  `ApiError` naming the missing role.
- **Where it runs:** the mock backend (`src/mocks/backend/core.ts`) is the only caller. Nothing else
  in the app looks accounts up by code.

## 3. Subtypes (drive the statements)

| Subtype                                            | Used by                                  |
| -------------------------------------------------- | ---------------------------------------- |
| `cash`, `bank`, `clearing`                         | cash-flow statement, "cash position" KPI |
| `receivable`, `payable`                            | aging, party ledgers                     |
| `inventory`, `tax`, `prepaid`, `otherCurrentAsset` | current assets                           |
| `fixedAsset`, `accumulatedDepreciation`            | non-current assets                       |
| `currentLiability`, `longTermLiability`            | balance-sheet split                      |
| `equity`                                           | equity                                   |
| `revenue`, `otherIncome`                           | P&L                                      |
| `costOfSales`                                      | gross profit                             |
| `operatingExpense`, `otherExpense`, `zakatTax`     | P&L sections                             |

## 4. Standard template (قياسي — recommended default, about 60 postable accounts)

Roles are in brackets. **Bold** marks an account that's new vs v1.

```text
1  الأصول
   11 الأصول المتداولة
      1110 الصندوق — الفرع الرئيسي                 [cash]   (+1111, 1112… one per branch)
      **1115 العهد النقدية وسلف الموظفين**
      1120 البنك — الحساب الجاري                   [bank]   (+1121… per bank / currency)
      **1125 مدى والبطاقات تحت التسوية**            [cardClearing]
      **1126 المحافظ الإلكترونية تحت التسوية**       [walletClearing]
      1130 العملاء                                 [receivable]  requiresParty
      1140 المخزون                                 [inventory]   allowManual=false
      **1145 بضاعة بالطريق بين الفروع**              [inventoryInTransit]
      1150 ضريبة القيمة المضافة — مدخلات           [vatInput]    allowManual=false
      **1160 دفعات مقدمة للموردين**
      **1170 مصروفات مدفوعة مقدماً**
      **1190 مخصص الديون المشكوك في تحصيلها**       (contra, CREDIT)
   12 الأصول غير المتداولة
      **1210 أثاث وديكورات**
      **1220 أجهزة ومعدات**
      **1230 سيارات**
      **1240 تحسينات على مبانٍ مستأجرة**
      **1290 مجمع الإهلاك**                          (contra, CREDIT)
2  الالتزامات
   21 الالتزامات المتداولة
      2100 الموردين                                [payable]     requiresParty
      2150 ضريبة القيمة المضافة — مخرجات           [vatOutput]   allowManual=false
      **2155 ضريبة القيمة المضافة — صافي مستحق**      [vatPayable]
      **2160 رواتب مستحقة**
      **2170 مصروفات مستحقة**
      **2180 دفعات مقدمة وأرصدة دائنة للعملاء**       [customerAdvances]
   22 الالتزامات غير المتداولة
      **2210 قروض طويلة الأجل**
      **2220 مخصص مكافأة نهاية الخدمة**
3  حقوق الملكية
      3100 رأس المال                               [capital]
      **3150 جاري المالك / الشركاء**                  [ownerCurrent]
      3250 الأرباح المحتجزة                         [retainedEarnings]
      3300 صافي ربح الفترة (افتراضي — يُحسب)         [currentEarnings]  virtual, never posted
      3400 المسحوبات الشخصية                        [drawings]   (DEBIT)
      **3900 أرصدة افتتاحية (مؤقت)**                  [openingBalanceEquity]
4  الإيرادات
      4100 مبيعات البضائع                           [sales]
      **4110 إيرادات الخدمات**                        [serviceRevenue]
      4200 مرتجعات ومسموحات المبيعات                [salesReturns] (contra, DEBIT)
      4300 إيرادات أخرى                             [otherIncome]
      **4310 أرباح فروق العملة**                      [fxGain]
      **4320 خصم مكتسب من الموردين**                  [purchaseDiscounts]
      **4330 زيادة الصندوق**                          [cashOver]
5  تكلفة المبيعات
      5100 تكلفة البضاعة المباعة                    [cogs]
      5110 فروقات جرد المخزون                       [inventoryVariance]  (was 4400 + 5800)
      **5120 بضاعة تالفة ومنتهية الصلاحية**           [inventoryWriteOff]
      **5130 شحن وتخليص المشتريات**                   [freightIn]  (when not capitalized)
6  المصروفات
   61 مصروفات البيع والتسويق
      **6110 عمولات ومكافآت البيع**
      **6120 دعاية وإعلان**
      **6130 مواد تغليف وأكياس**
      **6140 عمولات البطاقات ونقاط البيع**            [cardFees]
      **6150 مصاريف توصيل**
   62 المصروفات العمومية والإدارية
      6210 الرواتب والأجور
      **6215 التأمينات الاجتماعية**
      **6220 الإيجار**
      **6230 الكهرباء والمياه**
      **6240 الاتصالات والإنترنت**
      **6250 الصيانة والإصلاح**
      **6260 رسوم حكومية وتراخيص**
      **6270 أدوات مكتبية ومطبوعات**
      **6280 رسوم بنكية**                             [bankFees]
      **6290 مصروف الإهلاك**                          [depreciation]
      **6295 ديون معدومة ومشكوك فيها**                [badDebt]
   63 مصروفات أخرى
      **6310 خسائر فروق العملة**                      [fxLoss]
      **6320 عجز الصندوق**                            [cashShort]
      6390 مصروفات متنوعة
   69 الزكاة والضرائب
      **6910 الزكاة**                                 [zakat]  (SA add-on)
```

**Removed vs v1:** 5100 _Purchases_ and 5150 _Purchase returns_ (they don't belong in a perpetual
system; the code 5100 is reused for COGS), 4400 _stocktake gains_ and 5800 _stocktake losses_
(merged into 5110), and 5300 / 5500 (split into the 6xxx accounts).

## 5. Other templates and add-ons

| Template                  | For                                      | Difference from the standard template                                                                                                                                                                          |
| ------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **مبسّط** (basic, ~25)    | a single shop, owner-run                 | one bank, no fixed-asset/loan/accrual accounts, expenses collapsed into 6 accounts, no header levels below the root                                                                                            |
| **قياسي** (standard, ~60) | the default                              | §4                                                                                                                                                                                                             |
| **مفصّل** (detailed, ~90) | an accountant-run business with branches | + cheques receivable/payable (1135/2110), gift vouchers (2185), a revenue account per branch, split salaries (basic/housing/transport), GOSI payable, EOSB expense, prepaid rent per branch, bank per currency |

**Country add-ons** (chosen in onboarding step 3):

- **SA:** VAT 15%, GOSI, zakat, end-of-service provision.
- **EG:** VAT 14%, social insurance, a _withholding tax_ payable account (ضريبة الخصم والإضافة).
- **AE:** VAT 5%, corporate tax payable.

**Business add-ons:**

- **Pharmacy:** 4120 مبيعات أدوية معفاة/صفرية (a separate revenue line for the VAT return),
  5120 is renamed to include "أدوية منتهية الصلاحية".
- **Clothing:** 4110 is renamed "إيرادات التعديل والخياطة".
- **Services:** 5140 تكلفة الخدمات المقدمة.

**Switching templates later:** it's only additive. Accounts are added by role, or by code if the
role is unused, and nothing is removed. This is the reference system's tier idea without its "tier
upgrade" machinery.

## 6. Rules the UI enforces

- **Codes:** a child's code must start with its parent's code, and codes are unique. The form
  suggests the next free code.
- **Moving accounts:** an account can't move under a different `kind`.
- **Headers:** a header with children can't become postable, and vice versa once posted to.
- **Deleting:** only an account with no postings and no role can be deleted; otherwise _deactivate_.
- **Pickers:** account pickers show `code — name` with the header path in grey, e.g. "المصروفات ›
  العمومية". Pickers filter to leaf accounts, and to `allowManual` accounts in manual entries.
- **Tree page:** expand/collapse all, a balance column (with period filter), "show zero balances"
  toggle, drag to re-parent (with validation), and an export of the tree to Excel.

## 7. Demo seed migration

The v2 seed is rebuilt with the standard template. Account ids become `acc-<code>` on the new
codes. `scripts/e2e_flows.py` is updated: the manual-journal check uses 6130 مواد تغليف instead
of 5300.
