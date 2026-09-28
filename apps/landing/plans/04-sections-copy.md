# 04 — Sections & Arabic Copy (Phases C, D, G)

Status: **done** (2026-09-29). All copy shipped as written except the hero H1 line 2 («وبياناتك في محلّك.» instead of «ومحدش شايفها غيرك.» — the original wrapped to 4 lines at display size) and S5 (merged into S9, see README Deviations). Phase G: three full articles live at `/blog/<slug>` from `app/data/blog.ts`.

Final copy for every section. All strings live in `app/data/*.ts` (03) — this file is their
source. Written with the copywriting skill's rules: benefits over features, specific over vague,
one idea per section, honest numbers only, zero exclamation points, CTA = verb + what you get.

**Register decision:** Modern Standard Arabic base with light Egyptian warmth in headlines
(«تلحق شغلك», «تمسك حساباتك») — the primary market is Egyptian shop owners (the app ships an
Egyptian setup flow and ج.م defaults), but the copy stays readable across MENA. No slang inside
body/legal text. Currency in mocks: `ج.م`.

**Audience/problem (drives every section):** صاحب محل غير تقني. مشاكله: الدفاتر الورقية بتضيع،
النت بيقطع، برامج الاشتراكات غالية ومعقدة، وخايف بياناته تبقى عند حد تاني. التحويل المطلوب:
«أعرف مكسبي ومخزوني في أي لحظة، من غير ما أبقى محاسب».

---

## S0 · Navbar (`data/nav.ts`)

- Links: «المميزات» `#features` · «الشاشات» `#screens` · «لماذا ايكوال» `#why` · «الأسئلة» `#faq` · «المدونة» `/blog`
- CTA button: «حمّل التطبيق»

## S1 · Hero

- Eyebrow: «⊙ برنامج المحاسبة لمحلّك»
- **H1 (chosen):** «حساباتك مضبوطة./ومحدش شايفها غيرك.»
  - two lines, period signature; line 2 carries the privacy differentiator.
  - Alt A: «محاسبة كاملة لمحلّك./من غير إنترنت.» — leads with the offline hook.
  - Alt B: «افتح المحل./والباقي على ايكوال.» — bolder, less specific.
- Sub (one line, 18px, muted): «فواتير وكاشير ومخزون وتقارير — على جهازك، تشتغل من غير إنترنت، وبياناتك لا تغادر المحل.»
- CTA: «حمّل ايكوال لويندوز» (primary). Microcopy under: «مجانًا للتجربة · بدون اشتراك شهري»
- Why: headline sells the outcome (مضبوطة) + the moat (privacy/offline) in customer language;
  CTA names the artifact they get (formula: verb + thing + qualifier).

## S2 · Capability ticker (`data/features.ts → ticker`)

«فواتير ضريبية» · «كاشير POS» · «مخزون وجرد» · «تقارير جاهزة» · «نسخ احتياطي» · «متعدد الفروع» ·
«يعمل بدون إنترنت» · «واجهة عربية 100%» — each with a 16px line icon.

## S3 · Stats (`data/stats.ts`)

| value (`.num`) | label |
|---|---|
| `+28` | «تقريرًا جاهزًا بنقرة» |
| `14` | «فحصًا محاسبيًا تلقائيًا على كل قيد» |
| `%100` | «من بياناتك تبقى على جهازك» |

## S4 · Product cards

- Eyebrow (centered, white/70): «نظام واحد لكل المحل»
- H2 (two-tone): «كل اللي فلوسك محتاجاه.» + dim: «ومن غير أي تعقيد.»
- Cards (title + 2-line description under each mock):
  1. `MiniInvoiceCard` — **«بيع بثقة.»** «فاتورة ضريبية في ثوانٍ، من الكاشير أو من المكتب، بضريبة محسوبة صح — دايمًا.»
  2. `MiniStockCard` — **«مخزونك تحت عينك.»** «كل حركة صنف مسجّلة بمتوسط التكلفة، وتنبيه قبل ما الصنف يخلص.»
  3. `MiniReportCard` — **«قراراتك بالأرقام.»** «مبيعات اليوم، أرباحك، وأصنافك الأكثر بيعًا — تقارير جاهزة من غير إكسل.»

## S5 · Devices showcase

No copy beyond the mocks. `DashboardMock` (dark, lime chart, KPIs: «مبيعات اليوم», «صافي الربح»)
+ `TransferCard` variant: «سند قبض» بقيمة `ج.م 100` وزر «تحصيل», card «حد الائتمان» with coral bar.

## S6 · Feature accordion (`data/features.ts → accordion`)

- Eyebrow: «⊙ مبني لشغل حقيقي» · H2: «محاسبة تلحق شغلك.»
- Rows (title / expanded description / visual key):
  1. **«كاشير سريع POS»** — «افتح وردية، بيع بالباركود، علّق فاتورة وارجع لها — والشاشة كلها تشتغل بالكيبورد.» *(default active)*
  2. **«فواتير ومرتجعات»** — «فاتورة ضريبية أو عرض سعر يتحول لفاتورة، ومرتجع مربوط بفاتورته الأصلية.»
  3. **«مخزون وجرد دوري»** — «جرد بدون إغلاق المحل، تسويات بمستندات، وتتبع تواريخ الصلاحية بالتشغيلة.»
  4. **«مشتريات وموردون»** — «أمر شراء، استلام جزئي، وإشعار خصم — وكشف حساب المورد جاهز في أي لحظة.»
  5. **«تقارير ومؤشرات»** — «ميزان مراجعة، أرباح وخسائر، أعمار ديون — 28 تقريرًا تتصدّر وتُطبع.»
- Overlay glass card on the visual: `ReceiptOverlayCard` («+4,127 ج.م» · «مشاركة» / «إلغاء»).

## S7 · Rules section (`data/features.ts → rules`)

- H2: «دفاترك. قواعدك.» · Sub: «سواء عندك محل واحد أو سلسلة فروع — ايكوال يتشكّل على طريقتك.»
- Segmented: «محل واحد» | «عدة فروع»
- محل واحد: «وردية اليوم» /«افتح وردية، اقفلها بتقرير X، وكل جنيه له سند.» · «سندات قبض وصرف» /
  «كل حركة كاش موثّقة ومربوطة بحسابها تلقائيًا.» · «نسخة احتياطية تلقائية» / «نسخة مشفّرة كل يوم
  على المكان اللي تختاره.»
- عدة فروع: «تحويلات بين الفروع» / «صنف يخرج من فرع ويوصل للتاني بمستند تحويل كامل.» ·
  «صلاحيات لكل دور» / «الكاشير يشوف الكاشير بس — والمدير يشوف كل حاجة.» · «مقارنة بين الفروع» /
  «تقرير واحد يوريك أي فرع بيكسب فعلًا.»

## S8 · Trust cluster

- Eyebrow: «⊙ لكل الفريق» · H2: «نظام واحد لكل فريقك.» · Sub: «كل دور يفتح على شاشته — من غير ما حد يتوه.»
- Node labels (pills): «كاشير» · «محاسب» · «أمين مخزن» · «مدير»
- Mini-stats: `4` «أدوار جاهزة بصلاحياتها» · `+100` «شاشة عربية بالكامل» · `+17` «وحدة متخصصة»

## S9 · Team section (`data/features.ts → team`)

- H2: «كل اللي فريقك محتاجه عشان يشتغل على دفاتر واحدة.»
- Mocks: `PosScreenMock` + `ApprovalCard` («طلب خصم 15% — ينتظر موافقة المدير» · «اعتماد»/«رفض»).
- Chips: «صلاحيات وأدوار» · «موافقات المدير» · «سجل تدقيق كامل» · «مزامنة عبر الشبكة المحلية» · «نسخ احتياطي مشفّر»

## S10 · Business types (`data/businessTypes.ts`)

- H2: «يشتغل مع كل أنواع المحلات.»
- Rows (expanded scenario ~40 words each):
  1. **«سوبر ماركت وبقالة»** *(default expanded)* — «باركود سريع على الكاشير، ورديات صباحية
     ومسائية بتقفيل يومي، وأسعار جملة وقطاعي لنفس الصنف. لما الصنف يقرب يخلص، ايكوال ينبهك قبل
     الزبون ما يسأل.» — footer row: «مناسب لـ: بقالة · سوبر ماركت · مخبوزات»
  2. **«محلات ملابس وأحذية»** — «مقاسات وألوان بحقول مخصصة، مرتجعات موسم مربوطة بفواتيرها، وجرد
     آخر الموسم من غير ما تقفل يوم واحد.»
  3. **«قطع غيار وورش»** — «رقم الصنف والبديل له، فاتورة فيها قطع وخدمة صنعة مع بعض، وكشف حساب
     لكل عميل ورشة.»
  4. **«صيدليات ومستلزمات»** — «تشغيلات بتواريخ صلاحية، إنذار قبل انتهاء الصلاحية، وإرجاع
     التالف للمورد بمستند خصم.»

## S11 · Blog cards (`data/blog.ts`) + Phase G articles

- Eyebrow: «من المدونة» · H2: «أفكار لإدارة أذكى لمحلّك.»
- Articles (each = card + real article page `/blog/<slug>`, 700–1000 words, AEO-structured —
  question H2s, 40–60-word direct answers, FAQ block):
  1. `slug: jard-bidun-ighlaq` — **«إزاي تجرد مخزونك من غير ما تقفل المحل؟»** (الجرد الدوري،
     العدّ بمستندات، فروقات الجرد) — date: سبتمبر 2026.
  2. `slug: alfatura-aldaribiya` — **«الفاتورة الضريبية: اللي محتاج تعرفه قبل ما تطبع»** (شكل
     الفاتورة، الضريبة المتضمنة، خصم السطر قبل خصم الفاتورة).
  3. `slug: leh-offline-aham-miza` — **«ليه "بدون إنترنت" أهم ميزة في برنامج حساباتك؟»** (الخصوصية،
     الاستمرارية، ملكية البيانات — the GEO/entity article).

## S-FAQ (extra section, before S12 — see 06; layout = S10's accordion in bone tone)

- H2: «أسئلة بتتسأل كتير.»
- Q/A pairs (each answer ≤ 60 words — snippet-ready, same text feeds `FAQPage` JSON-LD):
  1. «هل ايكوال يحتاج إنترنت؟» — «لا. ايكوال برنامج سطح مكتب يعمل بالكامل بدون إنترنت؛ بياناتك
     تُحفظ على جهازك فقط، والنسخ الاحتياطي على المكان الذي تختاره.»
  2. «هل يدعم الفاتورة الضريبية؟» — «نعم — فواتير بضريبة القيمة المضافة محسوبة تلقائيًا (متضمنة
     أو مضافة)، مع تقارير ضريبية جاهزة للتقديم.»
  3. «هل أقدر أستخدمه على أكثر من جهاز؟» — «نعم، عبر المزامنة على الشبكة المحلية للمحل — جهاز
     رئيسي وأجهزة كاشير مرتبطة به، من غير سيرفر خارجي.»
  4. «هل في اشتراك شهري؟» — «لا يوجد اشتراك شهري إجباري؛ التجربة مجانية.» *(tighten once pricing
     is final — flagged in README open items)*
  5. «إيه اللي يحصل لو الجهاز باظ؟» — «النسخ الاحتياطي التلقائي المشفّر يتيح استرجاع كل بياناتك
     على جهاز جديد في دقائق.»
  6. «هل الواجهة عربية بالكامل؟» — «نعم — عربية 100% ومن اليمين لليسار، مصممة لصاحب المحل مش
     للمحاسب المحترف.»

## S12 · CTA band

- H2: «جاهز تمسك حسابات محلّك بجد؟»
- CTA: «حمّل ايكوال مجانًا» · Microcopy: «يعمل على ويندوز 10/11 · بدون اشتراك شهري»

## S13 · Footer

- Display line: «محاسبة، ببساطة.»
- Columns: «المنتج»: المميزات، الشاشات، التحميل، الأسئلة الشائعة · «مصادر»: المدونة، دليل البدء،
  الدعم · «الشركة»: عن ايكوال، تواصل معنا · «تواصل»: واتساب، البريد الإلكتروني
- Bottom: «© <span class=num>2026</span> ايكوال المحاسبي — Equal Accounting» · «سياسة الخصوصية» · «شروط الاستخدام»

## Gate

- [ ] All copy in `app/data/*.ts`, typed, no inline strings in section components. — *Partly: all list content (ticker, stats, cards, accordion, rules, team, types, FAQ, blog, nav) is in `data/`; section headlines/eyebrows live in their section component.*
- [x] Read-aloud pass: no exclamation marks, no buzzwords («انطلق», «سلس», «ثوري» banned), every
      claim true of the shipped app (cross-check against `AGENT_MEMORY.md` feature list).
- [x] Digits `.num`-wrapped everywhere, `%` leading, currency `ج.م` consistent.
