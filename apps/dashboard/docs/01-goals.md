# 01 — Goals

## Users

| User | Area | Needs |
|---|---|---|
| **Owner (you)** | admin | See the whole business at a glance, approve payments fast, change plans and prices, ship updates safely, find and fix errors |
| **Support / finance staff** | admin | Help a customer (their devices, subscription, diagnostics) or review payments, without access to what they don't need |
| **Shop owner** (non-technical, often on a phone) | portal | Understand the plans, pay the way they already pay (InstaPay / Vodafone Cash), and get the app activated without typing codes |

## Goals

| # | Goal | Measured by |
|---|---|---|
| G1 | Activation is one click from the desktop: open → login/signup → «تأكيد» → back in the app | Staging E2E, under 60 s when already logged in |
| G2 | Paying is familiar: clear manual payment instructions and a receipt upload that works on a phone | Portal flows usable at 360 px width |
| G3 | Payments get approved within minutes: the queue with receipt preview is one screen, one click | Admin review time |
| G4 | The owner changes prices and limits without a developer (plan version editor) | Publishing a version needs no code change |
| G5 | Calm and consistent: same tokens, RTL behavior and component language as the desktop app | UI review against `05-ui-rules.md` |

## Non-goals

- No accounting data and no reports about the customer's sales. That stays in the desktop app.
- No marketing site. The landing page is `apps/landing`. The public `/pricing` page here is for signed-out portal visitors.
- No card checkout at launch (Paymob comes later).
- No mobile app (`apps/mobile-app` is a future LAN-sync project).
