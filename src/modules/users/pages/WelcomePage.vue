<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { Moon, Store, Sun, Sparkles, Building2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import { resolvedTheme, toggleTheme } from '@/modules/core/controllers/useTheme';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { seedDatabase, seedEmptyCompany } from '@/mocks/seed';
import { flushSnapshot } from '@/mocks/persist';

const router = useRouter();
const pending = ref<'demo' | 'fresh' | null>(null);
const error = ref('');

async function startDemo() {
  error.value = '';
  pending.value = 'demo';
  try {
    seedDatabase();
    await flushSnapshot();
    router.replace('/login');
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    pending.value = null;
  }
}

async function startFresh() {
  error.value = '';
  pending.value = 'fresh';
  try {
    // TODO(phase 5): replace with the 11-step onboarding wizard from docs/v2/05-onboarding.md.
    // This is a minimal stub: an empty CoA + one admin user, just enough to reach /login.
    seedEmptyCompany();
    await flushSnapshot();
    router.replace('/login');
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    pending.value = null;
  }
}

</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-background px-6 py-10">
    <button
      type="button"
      class="fixed end-4 top-4 flex size-8 items-center justify-center rounded-md text-text-secondary hover:bg-surface-hover"
      aria-label="تبديل المظهر"
      @click="toggleTheme"
    >
      <Sun v-if="resolvedTheme === 'dark'" class="size-4" />
      <Moon v-else class="size-4" />
    </button>

    <div class="w-full max-w-2xl">
      <div class="mb-10 flex flex-col items-center text-center">
        <div class="mb-4 flex size-12 items-center justify-center rounded-xl bg-primary text-on-primary">
          <Store class="size-6" />
        </div>
        <h1 class="text-xl font-semibold">مرحباً بك</h1>
        <p class="mt-1 text-body text-text-secondary">نظام المحاسبة ونقاط البيع — اختر كيف تبدأ</p>
      </div>

      <p v-if="error" class="mb-4 rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger" role="alert">
        {{ error }}
      </p>

      <div class="grid gap-4 sm:grid-cols-2">
        <button
          type="button"
          class="group flex flex-col items-start gap-3 rounded-xl border border-border bg-surface p-6 text-start transition-colors hover:border-primary/50 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="pending !== null"
          @click="startFresh"
        >
          <span class="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Building2 class="size-5" />
          </span>
          <span class="text-sm font-semibold">ابدأ شركتك</span>
          <span class="text-xs leading-relaxed text-text-secondary">
            أنشئ شركة جديدة فارغة بدليل حسابات ومستخدم مدير، ثم أكمل الإعداد لاحقاً.
          </span>
          <AppButton variant="primary" size="sm" class="mt-1" :loading="pending === 'fresh'" :disabled="pending !== null">ابدأ الآن</AppButton>
        </button>

        <button
          type="button"
          class="group flex flex-col items-start gap-3 rounded-xl border border-border bg-surface p-6 text-start transition-colors hover:border-primary/50 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="pending !== null"
          @click="startDemo"
        >
          <span class="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Sparkles class="size-5" />
          </span>
          <span class="text-sm font-semibold">استكشف ببيانات تجريبية</span>
          <span class="text-xs leading-relaxed text-text-secondary">
            حمّل متجراً تجريبياً كاملاً (منتجات، عملاء، فواتير وحسابات) لتجربة النظام فوراً.
          </span>
          <AppButton variant="secondary" size="sm" class="mt-1" :loading="pending === 'demo'" :disabled="pending !== null">تحميل البيانات</AppButton>
        </button>
      </div>
    </div>
  </div>
</template>
