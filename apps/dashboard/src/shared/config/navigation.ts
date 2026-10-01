import type { Component } from "vue";
import type { AppRoute } from "@/shared/types/route";

export interface NavItem {
  label: string;
  to: AppRoute;
  icon?: Component;
}

export interface NavGroup {
  group: string;
  items: NavItem[];
}

/**
 * Placeholder admin sidebar structure (D1). Real wiring happens per module in D2 — each module adds
 * its own group/items here once its routes exist (docs/03-architecture.md's module table), rather
 * than hard-coding nav elsewhere. Keep group names matching that table.
 */
export const adminNavigation: NavGroup[] = [
  {
    group: "الرئيسية",
    items: [{ label: "لوحة التحكم", to: { name: "admin-home" } }],
  },
  {
    group: "المنشآت",
    items: [
      // { label: 'المنشآت', to: { name: 'admin-orgs' } }, // D2: organizations module
    ],
  },
  {
    group: "الفواتير والمدفوعات",
    items: [
      // { label: 'المدفوعات', to: { name: 'admin-payments' } }, // D2: payments module
    ],
  },
  {
    group: "الإصدارات",
    items: [
      // { label: 'الإصدارات', to: { name: 'admin-releases' } }, // D2: releases module
    ],
  },
];
