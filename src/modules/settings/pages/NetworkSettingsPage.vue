<script setup lang="ts">
/**
 * Settings → الشبكة وقاعدة البيانات (21 · 03.01 §6, Part 02 handoff §9): the Main-PC LAN-sharing
 * toggle, pairing-code display and connected-terminals count. Dormant/empty outside
 * `usesRust('settings')` — there is nothing to toggle against the mock backend.
 */
import { computed, onMounted, ref } from 'vue';
import { Laptop, RefreshCw, ShieldCheck, Wifi } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import SettingsPage from '@/modules/core/components/layouts/SettingsPage.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { ApiError, usesRust } from '@/modules/core/services/backend';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import * as networkService from '../services/networkService';
import type { LanSharingStatus } from '../types/network';

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();

const available = usesRust('settings');
const canWrite = computed(() => auth.can('settings', 'write'));

const loading = ref(true);
const status = ref<LanSharingStatus | null>(null);
const toggling = ref(false);
const rotating = ref(false);

async function load() {
  if (!available) {
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    status.value = await networkService.getLanSharingStatus();
  } catch (err) {
    toast.error(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

const isMain = computed(() => status.value?.role === 'main');
const lanSharing = computed({
  get: () => status.value?.lanSharing ?? false,
  set: (v: boolean) => void onToggle(v),
});

async function onToggle(enable: boolean) {
  toggling.value = true;
  try {
    if (enable) {
      await networkService.enableLanSharing();
      toast.success('تم تفعيل مشاركة قاعدة البيانات على الشبكة', 'سيطلب ويندوز إذناً مرة واحدة');
    } else {
      await disableWithConfirm(false);
    }
    await load();
  } catch (err) {
    toast.error(err);
  } finally {
    toggling.value = false;
  }
}

async function disableWithConfirm(alreadyConfirmed: boolean) {
  try {
    await networkService.disableLanSharing(alreadyConfirmed);
    toast.success('تم إيقاف مشاركة قاعدة البيانات على الشبكة');
  } catch (err) {
    if (err instanceof ApiError && err.code === 'CONFLICT') {
      const ok = await confirm({
        title: 'أجهزة الكاشير متصلة الآن',
        message: err.message,
        confirmText: 'إيقاف على أي حال',
        danger: true,
      });
      if (ok) {
        await networkService.disableLanSharing(true);
        toast.success('تم إيقاف مشاركة قاعدة البيانات على الشبكة');
        return;
      }
      // Revert the switch visually by reloading the real status.
      await load();
      return;
    }
    throw err;
  }
}

async function rotateCode() {
  const ok = await confirm({
    title: 'تغيير رمز الاقتران؟',
    message: 'ستحتاج كل أجهزة الكاشير إلى الاقتران من جديد بالرمز الجديد.',
    confirmText: 'تغيير الرمز',
    danger: true,
  });
  if (!ok) return;
  rotating.value = true;
  try {
    await networkService.rotatePairingCode();
    toast.success('تم تغيير رمز الاقتران');
    await load();
  } catch (err) {
    toast.error(err);
  } finally {
    rotating.value = false;
  }
}
</script>

<template>
  <SettingsPage title="الإعدادات" subtitle="الشبكة وقاعدة البيانات">
    <template #nav><SettingsTabs /></template>

    <EmptyState v-if="!available" title="متاح في نسخة سطح المكتب" description="مشاركة قاعدة البيانات على الشبكة تعمل فقط داخل تطبيق سطح المكتب المثبّت." :icon="Wifi" />

    <div v-else-if="loading" class="text-body text-text-secondary">جارٍ التحميل…</div>

    <div v-else class="max-w-2xl space-y-5">
      <!-- بطاقة الدور -->
      <AppCard>
        <div class="flex items-center gap-3">
          <span class="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
            <Laptop class="size-4" />
          </span>
          <div class="min-w-0">
            <p class="text-body font-medium text-text-primary">
              {{ isMain ? 'هذا الجهاز هو الجهاز الرئيسي' : `جهاز كاشير متصل بـ ${status?.mainHost ?? '—'}` }}
            </p>
            <p class="text-xs text-text-secondary">
              {{ isMain ? 'يستضيف قاعدة البيانات لبقية أجهزة الكاشير في هذا الفرع.' : 'يتصل بقاعدة بيانات الجهاز الرئيسي عبر الشبكة المحلية.' }}
            </p>
          </div>
        </div>
      </AppCard>

      <template v-if="isMain">
        <!-- تفعيل المشاركة -->
        <AppCard title="مشاركة قاعدة البيانات">
          <AppSwitch
            v-model="lanSharing"
            label="السماح لأجهزة الكاشير بالاتصال بهذا الجهاز"
            description="سيطلب ويندوز إذناً مرة واحدة لفتح المنفذ في جدار الحماية"
            :disabled="!canWrite || toggling"
          />
        </AppCard>

        <!-- بطاقة الاقتران -->
        <AppCard v-if="status?.lanSharing && status.pairing" title="اقتران أجهزة الكاشير">
          <div class="space-y-3 text-body">
            <div class="grid gap-3 sm:grid-cols-2">
              <div>
                <p class="field-label">اسم الجهاز</p>
                <p class="num" dir="ltr">{{ status.pairing.hostName }}</p>
              </div>
              <div>
                <p class="field-label">المنفذ</p>
                <p class="num" dir="ltr">{{ status.pairing.port }}</p>
              </div>
              <div class="sm:col-span-2">
                <p class="field-label">عناوين الشبكة</p>
                <p class="num" dir="ltr">{{ status.pairing.addresses.join('، ') || '—' }}</p>
              </div>
              <div class="sm:col-span-2">
                <p class="field-label">رمز الاقتران</p>
                <p class="num text-heading-sm" dir="ltr">{{ status.pairing.code }}</p>
              </div>
            </div>

            <div class="flex items-center justify-between border-t border-border pt-3">
              <p class="flex items-center gap-2 text-xs text-text-secondary">
                <ShieldCheck class="size-4" />
                أجهزة كاشير متصلة الآن: <span class="num">{{ status.connectedTerminals }}</span>
              </p>
              <AppButton size="sm" :icon="RefreshCw" :disabled="!canWrite" :loading="rotating" @click="rotateCode">تغيير رمز الاقتران</AppButton>
            </div>
          </div>
        </AppCard>
      </template>
    </div>
  </SettingsPage>
</template>
