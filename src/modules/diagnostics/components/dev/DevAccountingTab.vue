<script setup lang="ts">
/** Stub (18.B5) — posting traces + invariant results are Phase F ("Accounting debugger"), not this
 * phase. The `accounting` channel already exists in `logService` (traces gated behind debug mode,
 * invariant failures always logged), so this tab just says what is coming instead of staying hidden. */
import { FlaskConical } from '@lucide/vue';
</script>

<template>
  <div class="flex flex-col items-center gap-2 rounded-xl border border-dashed border-border py-16 text-center text-text-secondary">
    <FlaskConical class="size-6" />
    <p class="text-body">مصحح الحسابات (تتبع الترحيل والقواعد المحاسبية) قادم في المرحلة F.</p>
    <p class="text-xs">قناة <span class="num">accounting</span> مفعّلة بالفعل في نظام التسجيل — هذه الصفحة ستعرض تتبعاتها لاحقاً.</p>
  </div>
</template>
