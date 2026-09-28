# 01 — Reference Analysis (pixel anatomy of the Nero template)

> **Corrected during implementation (2026-09-29)** — see README "Deviations" X1–X3: S5 is not a
> separate section (it is a presentation shot of S9), the page is rounded white panels on a
> coral → forest frame (1400px frame, 1320px content, ~28px panel radius), S9's features are a
> hairline 3 + 2 grid, and S10's collapsed rows are light bars. Measured colors: coral `#F3553B`,
> forest `#03201F`, caption cream `#FFE9D4`, panels `#F9F9F9` / `#EEEEF0`.

Sources: `../reference/01-256119823.png` (high-res crops: hero, ticker, stats, product cards,
device showcase, feature accordion) and `../reference/03-256119823.png` (the full page,
1600×11507 — the authoritative section order and proportions).

All measurements below are normalized to a **1440px design frame** (the reference full-page shot is
1600px wide with the content column ≈ 1200px; treat the content max-width as **1200px** with 24px
gutters). "Mirror" notes describe the RTL adaptation. Every section lists: *Layout · Look · How it
works (behavior) · RTL · Equal content mapping*.

---

## Global observations (apply everywhere)

- **Palette** (measured off the crops):
  - Coral primary `#F4553C` (stat band, CTAs, eyebrows); hover/deep coral `#E14830`.
  - Bone/off-white page bg `#F1EFEA`; hero panel warm gradient `#F7F3EC → #EDE7DC` with coral glow bottom-right.
  - Ink (headings on light) `#182721` — a green-black, NOT pure black.
  - Forest dark sections/footer `#122019`; deeper footer wells `#0D1813`.
  - Lime accent inside dark app-UI mocks `#D3F36B`.
  - Muted body text on light `#6F746C` (~55% ink); on coral, white at 70–75%; on forest, white at 60%.
- **Type:** one grotesque family everywhere. Headings weight ~500, tight leading (1.02–1.1),
  tight tracking (Latin only — never track Arabic, D2). Eyebrow labels ~12px uppercase, letterspaced,
  coral (or white/70 on coral bg), always preceded by a small ✳/⊙ glyph icon.
  Sentence-case headlines that **end with a period** — a signature of this template. Keep the
  period in Arabic (`.محاسبة مبنية حولك` renders as «محاسبة مبنية حولك.»).
- **Two-tone headline trick:** second sentence of a headline rendered at 45–55% opacity
  ("Everything your money needs. *Nothing it doesn't.*"). Reuse in Arabic.
- **Shape language:** pills (fully rounded) for all buttons/nav; cards radius 24px; large media
  panels 28–32px; mini app-UI cards 16–20px. Hairline separators everywhere (1px, 15–25% opacity),
  almost no shadows except soft ambient on floating UI cards.
- **Section rhythm:** vertical padding ≈ 112–128px desktop; headline → content gap ≈ 56–64px.
- **Backgrounds alternate:** bone → coral (big block) → bone → forest → bone → forest (CTA+footer).

---

## S0 · Navbar

- **Layout:** floating white pill bar, inset ~24px from top, width = content width (1200px),
  height ~56px, radius full. Left: logo mark (coral glyph, ~24px). Center: 5 text links
  (Personal · Business · Features · Security · About), 14px, ink, gap ~28px. Right: `Login`
  coral pill button (~36px tall, px-20).
- **How it works:** position `fixed`/`sticky` over the hero; on scroll it stays pinned with a subtle
  shadow/backdrop-blur once scrolled (standard template behavior); nav links are anchor-scrolls to
  sections. Mobile: logo + hamburger → full-screen sheet.
- **RTL:** logo sits at the **start (right)**, button at the **end (left)** — automatic with logical
  properties/flex; never hand-mirror.
- **Equal:** logo = Equal mark + «ايكوال»; links: «المميزات · الشاشات · لماذا ايكوال · الأسئلة · المدونة»;
  button: «حمّل التطبيق». All anchors on-page (`#features`, `#screens`, `#why`, `#faq`, `/blog`).

## S1 · Hero

- **Layout:** one huge rounded panel (radius ~28px) filling the content width, height ≈ 640–700px,
  **framed by the coral section behind it** — the coral of S2/S3 begins behind the hero's bottom
  edge, and a thin coral sliver shows on the left edge in the crop (the panel sits on a coral
  ground). Inside the panel: text block bottom-left third — eyebrow (`⊙ THE FUTURE OF BANKING`,
  coral, 12px), H1 two lines (`Banking, Built / Around You.`, ~72px/1.05, ink), then a single
  coral pill CTA (`Open an Account`, ~48px tall). Right two-thirds: abstract 3D visual — brushed
  warm-metal curved sheet, a **halftone dot sphere** fading out, coral glow bleeding from
  bottom-right corner.
- **How it works:** purely visual; CTA scrolls to download/CTA section. Visual is a static image
  with (in the live template) slow parallax drift on scroll.
- **RTL:** text block bottom-**right**; visual weight flows to the **left**; glow bottom-left.
  The composition mirrors entirely.
- **Equal:** eyebrow «⊙ برنامج المحاسبة لمحلّك»; H1 «حساباتك، مضبوطة. / من غير إنترنت.» (final copy in
  04); CTA «حمّل ايكوال لويندوز». Visual: same abstract warm-metal + halftone treatment (layered
  CSS gradients + SVG dot-matrix + prerendered abstract asset), with a floating Arabic invoice
  mini-card (glass) as an optional accent.
- **Assets:** `hero-abstract.webp` (prerendered, ~1600×900), SVG halftone pattern, noise texture.

## S2 · Capability ticker (reference: logo strip)

- **Layout:** full-bleed coral band directly under the hero, height ~96px, top & bottom 1px
  hairlines (white/25). Eight white wordmarks in a row, equal visual weight, separated by
  **vertical hairlines** running the band's full height. Wordmarks ~20px, white.
- **How it works:** infinite marquee, slow constant velocity, seamless loop; pauses are NOT visible
  (no snap). Hover does not stop it in the reference.
- **RTL:** marquee travels **left→right** (content enters from the left edge, exits right — i.e.
  reading-direction reversed motion; in practice: run the same x-translation loop with positive
  direction).
- **Equal (D4):** feature wordmarks with tiny icons: «فواتير ضريبية · كاشير POS · مخزون وجرد ·
  تقارير جاهزة · نسخ احتياطي · وضع ليلي · بدون إنترنت · متعدد الفروع». Duplicate list 2× for the
  seamless loop.

## S3 · Stats band

- **Layout:** same coral, ~340px tall, 3 equal columns split by full-height vertical hairlines
  (white/25). Each column: giant numeral (~96px, white; the `+`/`%` glyph at ~50% white) centered,
  small uppercase label below (~12px, white/70, letterspaced — Arabic: 13px weight 500, no tracking).
- **How it works:** on-scroll count-up animation (0 → value) with once-only trigger.
- **RTL:** column order mirrors (first stat at the right). Numerals stay LTR (`.num`).
- **Equal (D5):** `+28` «تقريرًا جاهزًا» · `14` «فحصًا محاسبيًا تلقائيًا» · `%100` «بياناتك على جهازك».

## S4 · Product cards ("Everything your money needs.")

- **Layout:** continues the same coral block, ~880px tall. Centered eyebrow (`ONE BANKING
  PLATFORM`, white/70) → centered H2 two lines (~52px): line 1 white, trailing sentence white/50
  (two-tone trick). Below (~64px gap): **3 equal glass cards** (grid-cols-3, gap ~24px), each
  radius ~24px, background white/12 + 1px white/25 border (liquid-glass: add inner top highlight).
  Inside each: a **mini app-UI mock** (light warm-white card, radius 16px) showing: ① a
  transaction/share card, ② a savings goals list with progress bars, ③ an expenses bar-chart with
  period toggle. Below each card (outside it): bold white mini-title + 2-line white/70 description.
- **How it works:** cards reveal with stagger on scroll; the mini-UIs contain looping
  micro-animations in the live template (progress bars fill, numbers tick).
- **RTL:** grid order mirrors automatically; mini-UIs are RTL Arabic layouts; progress bars fill
  from the right.
- **Equal (D7):** ① فاتورة بيع (إجمالي، ضريبة، زر مشاركة) — «بيع بثقة.» ② حركة المخزون (أصناف +
  أشرطة كميات) — «مخزونك تحت عينك.» ③ لوحة تقارير مصغّرة (أعمدة أسبوعية + مبدّل مدة) — «قراراتك
  بالأرقام.» All three are Vue components (`MiniInvoiceCard.vue`, `MiniStockCard.vue`,
  `MiniReportCard.vue`).

## S5 · Device showcase (dark panel, crop 01 top-right)

- **Layout:** full-bleed **black/near-black** section (~760px), inside it a large white/very-light
  rounded panel (radius ~24px) with padding, containing **two grey inner panels** side by side
  (≈58%/42%): left = phone mockup (dark app UI: greeting, big balance `$50,320`, lime bar chart,
  bottom tab bar) floating on light-grey; right = stacked UI cards (Transfer Money card: avatar
  row, `$100`, mastercard chip row, coral `Send` pill; below it a Spending Limit card with coral
  progress bar). Panels radius ~20px, bg `#EDEDED`.
- **How it works:** static composition; in the live template the inner cards drift on
  scroll (light parallax at different speeds).
- **RTL:** swap panel order (phone panel starts from the right); cards' internal layout RTL.
- **Equal:** phone → **desktop app window mockup** (Equal is a desktop product): dark-theme Equal
  window (شاشة الرئيسية: KPIs + رسم مبيعات ليموني) at a slight angle; right panel: «سند قبض» card
  (`ج.م 100`, زر «تحصيل») + «حد الائتمان» card with coral progress. Screenshot slot behind a
  component frame (D7): `WindowMock.vue` renders the chrome; image slot inside.

## S6 · Feature accordion ("Banking that keeps up.")

- **Layout:** bone background, ~800px. Eyebrow coral (`BUILT FOR REAL LIFE`) + H2 ink (~52px),
  both **start-aligned**. Two columns (≈45%/55%, gap ~64px). Left: vertical **feature accordion**,
  5 rows separated by hairlines (ink/10): each row = small square-outline icon (⊙, 16px) + title
  (17px). The **active** row is coral (icon+title) and expands to show a 2-line muted description;
  inactive rows are ink at full height ~64px. Right: photo card (radius ~24px, ~520px tall,
  lifestyle photo) with a **glass transaction overlay card** (blurred dark glass, radius 16px)
  pinned bottom-left showing a transaction UI (`+4.127`, avatar, Share/Cancel pills, toggle row,
  status row).
- **How it works:** click a row → it becomes active (coral + description slides open) and the
  right visual **cross-fades** to that feature's image. Template also auto-advances every ~5s.
- **RTL:** accordion on the **right**, visual on the **left**; overlay card pins bottom-**right**
  of the photo; chevron/expansion motion mirrored.
- **Equal:** headline «محاسبة تلحق شغلك.»; 5 rows mapped to real modules:
  ① «كاشير سريع POS» (وردية، تعليق فاتورة، باركود) — active default; ② «فواتير ومرتجعات»;
  ③ «مخزون وجرد دوري»; ④ «مشتريات وموردون»; ⑤ «تقارير ومؤشرات». Right visual: Equal screenshot
  per feature (or styled mock) + glass overlay = Arabic receipt card (`+4,127 ج.م`, «مشاركة/إلغاء»).

## S7 · Dark rules section ("Your money. Your rules.")

- **Layout:** full-bleed forest `#122019`, ~760px. Start-aligned H2 white («Your money. Your
  rules.») + one-line white/60 sub. Under it, **two small toggle pills** (`PERSONALS` filled
  white/10 outline, `BUSINESS` ghost) — a segmented control. Below: two columns. Left: 3 stacked
  items separated by hairlines (white/12): title (18px white) + 2-line description (white/60).
  Right: large 3D visual — tilted glass/frosted **credit card** with brand wordmark, floating,
  soft glow.
- **How it works:** the segmented control switches the 3 items' content (personal vs business)
  with a fade/slide; card visual subtly rotates on scroll (parallax/tilt).
- **RTL:** items column right, visual left; pills order mirrored.
- **Equal:** H2 «دفاترك. قواعدك.»; toggle «محل واحد | عدة فروع». Items (per mode):
  محل واحد → «وردية اليوم» / «سندات قبض وصرف» / «نسخة احتياطية تلقائية»;
  عدة فروع → «تحويلات بين الفروع» / «صلاحيات لكل دور» / «تقارير مقارنة بين الفروع».
  Visual: 3D-tilted **Equal invoice card** (glass, Arabic) instead of a credit card.

## S8 · Trust cluster ("Banking millions can count on.")

- **Layout:** bone, ~720px, everything centered. Eyebrow + H2 (~48px) + one-line muted sub.
  Middle: **avatar constellation** — 6–8 circular photos (40–64px) scattered on a wide canvas,
  connected by thin lines; two small white pill labels with avatar («Customer», «Creative
  People») attached to nodes. Below: 3-column mini-stat row (numerals ~48px ink, muted labels),
  vertical hairlines between.
- **How it works:** avatars float gently (slow y-drift loops); labels pop in on scroll; count-up
  on the mini-stats.
- **RTL:** constellation mirrors (labels attach on mirrored sides); stats order mirrors.
- **Equal:** H2 «نظام واحد لكل فريقك.» — the nodes are Equal's real **roles**: labels «كاشير»،
  «محاسب»، «أمين مخزن»، «مدير». Stats: `4` أدوار جاهزة · `+100` شاشة عربية · `+17` وحدة متخصصة.

## S9 · Team section ("Everything your team needs…")

- **Layout:** bone continues, ~980px. Big start-aligned H2 across ~65% width (~48px, two lines).
  Below: **two rounded grey panels** side by side (same treatment as S5's inner panels — bg
  `#EDEDED`, radius 20px, ≈55%/45%): left = dark phone/app mock again, right = transfer card mock.
  Below them: **feature chip rows** — 5 outlined pills (radius full, 1px ink/15 border, icon+label,
  ~44px tall) arranged 3 + 2, centered.
- **How it works:** chips reveal with stagger; panels have hover-lift (subtle).
- **RTL:** panel order + chip order mirror.
- **Equal:** H2 «كل ما يحتاجه فريقك للشغل على دفاتر واحدة.» Panels: ① لقطة «شاشة الكاشير» (مكوّن
  مُعاد بناؤه بالعربية) ② بطاقة «موافقة مدير» (طلب خصم → زرا اعتماد/رفض). Chips: «صلاحيات
  وأدوار» · «موافقات المدير» · «سجل تدقيق كامل» · «مزامنة عبر الشبكة المحلية» · «نسخ احتياطي مشفّر».

## S10 · Business-type accordion (reference: testimonials "Relied on by startups and enterprises")

- **Layout:** bone, ~640px. Start-aligned H2 (~44px, two lines). Below: **vertical accordion of
  wide bars** (full content width): the expanded row is a light card (radius 20px, generous
  padding) = logo top-start, quote paragraph (~20px, ink, ~60% width), author row (avatar 40px +
  name + role muted); collapsed rows = slim dark-forest bars (radius 16px, ~72px) with white logo
  start + `+` icon end, stacked with 12px gaps.
- **How it works:** click a collapsed bar → it expands (height auto animation), previous one
  collapses (single-open accordion).
- **RTL:** logo at start (right), `+` icon at end (left); expansion animation unchanged.
- **Equal (D6):** H2 «يشتغل مع كل أنواع المحلات.» Rows: «سوبر ماركت وبقالة» (expanded default —
  scenario: باركود، ميزان، ورديات…) · «محلات ملابس» (مقاسات وألوان عبر الحقول المخصصة…) ·
  «قطع غيار وورش» (أرقام أصناف وبدائل…) · «صيدليات ومستلزمات» (تواريخ صلاحية وتشغيلات…).

## S11 · Blog cards ("Ideas for a better financial future.")

- **Layout:** bone, ~760px. Centered eyebrow (`FROM THE JOURNAL`) + centered H2 (~44px). Grid of
  3 cards (gap 24px): each = image (4:3, radius 20px), title (18px ink, 2 lines), date row
  (13px muted). No card borders — image + text stack.
- **How it works:** cards link to articles; hover = image subtle zoom (scale 1.04) + title →
  coral.
- **RTL:** grid mirrors; date row `.num` for the date digits.
- **Equal:** eyebrow «من المدونة»; H2 «أفكار لإدارة أذكى لمحلّك.»; 3 real articles (G phase):
  ① «إزاي تجرد مخزونك من غير ما تقفل المحل» ② «الفاتورة الضريبية: اللي محتاج تعرفه قبل ما تطبع»
  ③ «ليه الأوفلاين أهم ميزة في برنامج حساباتك». Images: styled abstract/product shots (no stock
  people needed).

## S12 · CTA band

- **Layout:** full-bleed forest, ~420px, centered: H2 white two lines (~44px) + coral pill CTA
  (~52px). Nothing else — the calm before the footer.
- **How it works:** CTA = same download action as navbar. Gentle reveal.
- **Equal:** H2 «جاهز تمسك حسابات محلّك بجد؟» CTA «حمّل ايكوال مجانًا». Under-button microcopy
  (12px white/50): «يعمل على ويندوز 10/11 · بدون اشتراك شهري».

## S13 · Footer

- **Layout:** forest continues (hairline white/10 separates from S12). Top block: logo + brand
  name start-aligned, then a **huge display line** («Banking, made simple.» ~56px white) spanning
  the width. Middle: link columns grid (4 columns: Product / Resources / Company / Contact),
  13px white/60 links, 12px white/35 column titles. Bottom bar (hairline above): social icons
  start, legal links end (Privacy · Terms · Disclosures), 12px white/40.
- **RTL:** columns and bars mirror; social icons at the right.
- **Equal:** display line «محاسبة، ببساطة.»; columns: «المنتج» (المميزات، الشاشات، التحميل،
  الأسئلة الشائعة) · «مصادر» (المدونة، دليل البدء، الدعم) · «الشركة» (عن ايكوال، تواصل معنا) ·
  «تواصل» (واتساب، بريد). Legal: «سياسة الخصوصية · شروط الاستخدام». Bottom note: «© <span
  class="num">2026</span> ايكوال المحاسبي».

---

## Section → background map (for the page skeleton)

```
S0 navbar (fixed, white pill)
S1 hero          bone panel on coral ground
S2 ticker        coral  ┐
S3 stats         coral  │ one continuous coral block
S4 product cards coral  ┘
S5 devices       near-black (#0F0F0F) with light inner panel
S6 accordion     bone
S7 rules         forest
S8 trust         bone
S9 team          bone (grey inner panels)
S10 types        bone (forest collapsed bars)
S11 blog         bone
S12 CTA          forest ┐ one continuous forest block
S13 footer       forest ┘
```
