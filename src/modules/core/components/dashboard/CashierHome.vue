<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Banknote, Clock, ShoppingCart, Wallet } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { formatDateLong, formatNumber, formatTime } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { usePosStore } from '@/modules/invoices/controllers/usePosStore';
import { getCurrentShift, type ShiftRow } from '@/modules/invoices/services/invoiceService';

/**
 * v2 (docs/v2/01-personas.md §1, docs/v2/11 Part B "role homes" table): "Home = the POS itself;
 * the shift panel… is in the POS header". The cashier's dedicated home route still needs *some*
 * landing content though (it's what `/` shows before they open the POS), so this is a small "my
 * shift" summary + a big button into the POS — not a scaled-down version of the manager's home.
 */
const auth = useAuthStore();
const pos = usePosStore();

const shift = ref<ShiftRow | null>(null);
const loading = ref(true);

onMounted(async () => {
  shift.value = (await getCurrentShift(pos.terminalId)) ?? null;
  loading.value = false;
});

const greeting = computed(() => (new Date().getHours() < 12 ? 'صباح الخير' : 'مساء الخير'));
</script>

<template>
  <div>
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">{{ greeting }}، {{ auth.user?.name.split(' ')[0] }}</h1>
        <p class="mt-0.5 text-body text-text-secondary">{{ formatDateLong(new Date().toISOString()) }}</p>
      </div>
      <AppButton variant="primary" :icon="ShoppingCart" :to="{ name: 'pos' }">فتح نقطة البيع</AppButton>
    </div>

    <div class="grid gap-4 lg:grid-cols-3">
      <AppCard class="lg:col-span-2" title="ورديتي" padding="sm">
        <SkeletonBlock v-if="loading" :lines="4" />
        <EmptyState v-else-if="!shift" :icon="Clock" title="لا توجد وردية مفتوحة" description="افتح وردية من نقطة البيع لبدء البيع" compact>
          <AppButton size="sm" variant="primary" :to="{ name: 'pos' }">فتح وردية</AppButton>
        </EmptyState>
        <div v-else class="grid gap-4 sm:grid-cols-4">
          <div>
            <p class="text-xs text-text-secondary">مبيعات الوردية</p>
            <p class="mt-1 text-stat font-semibold"><MoneyText :value="shift.salesTotal" /></p>
          </div>
          <div>
            <p class="text-xs text-text-secondary">عدد الفواتير</p>
            <p class="mt-1 text-stat font-semibold"><span class="num">{{ formatNumber(shift.invoiceCount) }}</span></p>
          </div>
          <div>
            <p class="text-xs text-text-secondary">النقد المتوقع بالصندوق</p>
            <p class="mt-1 text-stat font-semibold"><MoneyText :value="shift.expectedCash" /></p>
          </div>
          <div>
            <p class="text-xs text-text-secondary">فُتحت</p>
            <p class="mt-1 text-body font-medium">{{ formatTime(shift.openedAt) }}</p>
          </div>
          <div class="sm:col-span-4 flex flex-wrap gap-2 border-t border-border pt-3">
            <AppButton size="sm" :icon="Banknote" :to="{ name: 'pos' }">إيداع/سحب نقدي</AppButton>
            <AppButton size="sm" :icon="Wallet" :to="{ name: 'pos-shifts' }">تقرير X</AppButton>
            <AppButton size="sm" variant="primary" :to="{ name: 'pos' }">إغلاق الوردية</AppButton>
          </div>
        </div>
      </AppCard>

      <AppCard title="الورديات" padding="none">
        <RouterLink :to="{ name: 'pos-shifts' }" class="flex items-center justify-between gap-3 px-4 py-3 hover:bg-surface-hover">
          <span class="text-body">عرض كل الورديات وتاريخها</span>
        </RouterLink>
      </AppCard>
    </div>
  </div>
</template>
