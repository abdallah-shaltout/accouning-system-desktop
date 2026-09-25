<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { LogIn, Moon, Sun } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import BrandLogo from '@/modules/core/components/ui/BrandLogo.vue';
import { resolvedTheme, toggleTheme } from '@/modules/core/controllers/useTheme';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { validate } from '@/modules/core/helpers/validation';
import { APP_NAME_AR } from '@/modules/core/helpers/brand';
import { useAuthStore } from '../controllers/useAuthStore';
import { getDemoAccounts } from '../services/authService';
import { loginSchema } from '../validators/userSchema';

const auth = useAuthStore();
const router = useRouter();
const route = useRoute();

const form = reactive({ username: '', password: '' });
const errors = ref<Record<string, string>>({});
const serverError = ref('');
const pending = ref(false);
const demo = ref<Awaited<ReturnType<typeof getDemoAccounts>>>([]);

onMounted(async () => {
  demo.value = await getDemoAccounts();
});

async function submit() {
  serverError.value = '';
  errors.value = validate(loginSchema, form);
  if (Object.keys(errors.value).length) return;
  pending.value = true;
  try {
    await auth.login(form.username, form.password);
    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/';
    // Cashiers land on the POS directly; the dashboard itself branches per role otherwise (storekeeper home, etc).
    router.replace(redirect === '/' && auth.role === 'cashier' ? '/pos' : redirect);
  } catch (err) {
    serverError.value = errorMessage(err);
  } finally {
    pending.value = false;
  }
}

function useDemo(account: { username: string; password: string }) {
  form.username = account.username;
  form.password = account.password;
  submit();
}
</script>

<template>
  <div class="flex min-h-screen bg-background">
    <div class="flex flex-1 flex-col items-center justify-center px-6 py-10">
      <div class="w-full max-w-[360px]">
        <div class="mb-8">
          <BrandLogo class="mb-5 h-9 w-auto" />
          <h1 class="text-lg font-semibold">تسجيل الدخول</h1>
          <p class="text-body text-text-secondary">{{ APP_NAME_AR }}</p>
        </div>

        <form class="space-y-4" novalidate @submit.prevent="submit">
          <AppInput v-model="form.username" label="اسم المستخدم" ltr autofocus :error="errors.username" />
          <AppInput v-model="form.password" label="كلمة المرور" type="password" ltr :error="errors.password" />
          <p v-if="serverError" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger" role="alert">
            {{ serverError }}
          </p>
          <AppButton type="submit" variant="primary" block size="lg" :loading="pending" :icon="LogIn">دخول</AppButton>
        </form>

        <div v-if="demo.length" class="mt-8">
          <p class="mb-2 text-xs text-text-secondary">حسابات تجريبية (بيانات وهمية)</p>
          <div class="overflow-hidden rounded-lg border border-border">
            <button
              v-for="d in demo"
              :key="d.username"
              type="button"
              class="flex w-full items-center justify-between gap-3 border-b border-border px-3 py-2 text-start text-body last:border-0 hover:bg-surface-hover"
              :disabled="pending"
              @click="useDemo(d)"
            >
              <span class="min-w-0">
                <span class="block truncate">{{ d.name }}</span>
                <span class="num block text-tiny text-text-secondary">{{ d.username }} / {{ d.password }}</span>
              </span>
              <span class="shrink-0 rounded-full bg-surface-hover px-2 py-0.5 text-tiny text-text-secondary">{{ ROLE_LABEL[d.role] }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <aside class="relative hidden w-[42%] max-w-xl flex-col justify-between border-s border-border bg-surface p-10 lg:flex">
      <button
        type="button"
        class="absolute end-4 top-4 flex size-8 items-center justify-center rounded-md text-text-secondary hover:bg-surface-hover"
        aria-label="تبديل المظهر"
        @click="toggleTheme"
      >
        <Sun v-if="resolvedTheme === 'dark'" class="size-4" />
        <Moon v-else class="size-4" />
      </button>
      <div />
      <div>
        <p class="text-2xl font-semibold leading-snug">
          مبيعاتك، مخزونك، وحساباتك<br />
          <span class="text-text-secondary">في تطبيق واحد على جهازك.</span>
        </p>
        <ul class="mt-6 space-y-2.5 text-body text-text-secondary">
          <li>• نقطة بيع سريعة مع فاتورة ضريبية ورمز QR</li>
          <li>• قيود محاسبية تلقائية بنظام القيد المزدوج</li>
          <li>• تقارير: ميزان المراجعة، قائمة الدخل، الميزانية، الضريبة</li>
        </ul>
      </div>
      <p class="text-xs text-text-secondary">نسخة واجهة تجريبية — البيانات وهمية وتُعاد عند إعادة التشغيل.</p>
    </aside>
  </div>
</template>
