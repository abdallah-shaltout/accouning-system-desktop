<script setup lang="ts">
import type { AuditEntry } from '../types';

/** Side-by-side field diff for one audit entry (18.B4) — only the changed fields, not the whole record. */
defineProps<{ entry: AuditEntry }>();

function display(v: unknown): string {
  if (v === undefined) return '—';
  if (v === null) return 'فارغ';
  if (typeof v === 'object') return JSON.stringify(v);
  return String(v);
}
</script>

<template>
  <div class="space-y-3 text-body">
    <div class="grid grid-cols-2 gap-3 text-xs text-text-secondary">
      <div><span class="font-medium text-text-primary">الكيان:</span> {{ entry.entity }} — <span class="num">{{ entry.entityId }}</span></div>
      <div><span class="font-medium text-text-primary">الإجراء:</span> {{ entry.action }}</div>
    </div>
    <p v-if="entry.reason" class="rounded-md bg-surface p-2 text-xs">السبب: {{ entry.reason }}</p>

    <div v-if="entry.after?.length" class="overflow-hidden rounded-md border border-border">
      <table class="w-full text-xs">
        <thead class="bg-surface">
          <tr>
            <th class="px-3 py-2 text-start font-medium">الحقل</th>
            <th class="px-3 py-2 text-start font-medium">قبل</th>
            <th class="px-3 py-2 text-start font-medium">بعد</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(f, i) in entry.after" :key="f.field" class="border-t border-border">
            <td class="px-3 py-1.5 font-medium">{{ f.field }}</td>
            <td class="px-3 py-1.5 text-danger num">{{ display(entry.before?.[i]?.before) }}</td>
            <td class="px-3 py-1.5 text-success num">{{ display(f.after) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-else class="text-xs text-text-secondary">لا توجد تفاصيل حقول لهذا الإجراء.</p>
  </div>
</template>
