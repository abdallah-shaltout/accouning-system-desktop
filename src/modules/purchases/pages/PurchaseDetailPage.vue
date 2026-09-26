<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRoute } from 'vue-router';
import { AlertTriangle, Ban, BookOpen, HandCoins, PackageCheck, Pencil, Printer, Send, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { isTauri } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { PAYMENT_METHOD_LABEL, PAYMENT_STATUS, PURCHASE_STATUS } from '@/modules/core/helpers/labels';
import { renderAndSave } from '@/modules/core/services/pdfService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { cancelPurchaseOrder, getPurchaseOrder, sendPurchaseOrderToSupplier } from '../services/purchaseService';

const route = useRoute('purchase');
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const id = String(route.params.id);

const { data, error, reload } = useAsync(() => getPurchaseOrder(id));
const po = computed(() => data.value);
const canWrite = computed(() => auth.can('purchases', 'write'));
const busy = ref<'cancel' | 'send' | null>(null);

/**
 * v2 phase 11b (docs/v2/12-documents-pdf-excel.md §3 "purchase order" now has a real template):
 * renders through `pdfService` in the desktop app, falling back to the v1 browser print route
 * only outside Tauri — same pattern `InvoiceDetailPage.vue`'s `print()` uses.
 */
async function printPurchaseOrder() {
  if (isTauri()) {
    const ok = await renderAndSave('purchaseOrder', id, `${po.value?.number ?? id}.pdf`);
    if (ok) return;
    toast.error('تعذر إنشاء ملف PDF');
    return;
  }
  router.push(`/print/purchases/${id}`);
}

async function doCancel() {
  const ok = await confirm({ title: `إلغاء ${po.value?.number}؟`, confirmText: 'إلغاء الأمر', cancelText: 'تراجع', danger: true });
  if (!ok) return;
  busy.value = 'cancel';
  try {
    await cancelPurchaseOrder(id);
    toast.success('تم إلغاء أمر الشراء');
    reload();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = null;
  }
}

async function doSend() {
  busy.value = 'send';
  try {
    await sendPurchaseOrderToSupplier(id);
    toast.success('تم إرسال أمر الشراء للمورد');
    reload();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = null;
  }
}
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="po ? `أمر شراء ${po.number}` : '…'" back="/purchases">
        <template v-if="po" #badge>
          <StatusBadge :tone="PURCHASE_STATUS[po.status].tone" :label="PURCHASE_STATUS[po.status].label" />
          <StatusBadge v-if="po.status === 'RECEIVED'" :tone="PAYMENT_STATUS[po.paymentStatus].tone" :label="PAYMENT_STATUS[po.paymentStatus].label" />
        </template>
        <template v-if="po" #subtitle>
          <RouterLink :to="`/suppliers/${po.supplierId}`" class="hover:text-primary">{{ po.supplierName }}</RouterLink> ·
          <span class="num">{{ formatDate(po.date) }}</span>
        </template>
        <template v-if="po && canWrite" #actions>
          <template v-if="po.status === 'DRAFT'">
            <AppButton variant="danger" :icon="Ban" :loading="busy === 'cancel'" @click="doCancel">إلغاء</AppButton>
            <AppButton :icon="Pencil" :to="`/purchases/${id}/edit`">تعديل</AppButton>
            <AppButton :icon="Send" :loading="busy === 'send'" @click="doSend">إرسال للمورد</AppButton>
            <AppButton variant="primary" :icon="PackageCheck" :to="`/purchases/${id}/receive`">استلام</AppButton>
          </template>
          <template v-else-if="po.status === 'ORDERED'">
            <AppButton variant="danger" :icon="Ban" :loading="busy === 'cancel'" @click="doCancel">إلغاء</AppButton>
            <AppButton :icon="Printer" @click="printPurchaseOrder">طباعة أمر الشراء</AppButton>
            <AppButton variant="primary" :icon="PackageCheck" :to="`/purchases/${id}/receive`">استلام</AppButton>
          </template>
          <template v-else-if="po.status === 'RECEIVED'">
            <AppButton :icon="Undo2" :to="`/purchases/${id}/return`">مرتجع للمورد</AppButton>
            <AppButton
              v-if="po.outstanding > 0 && auth.can('payments', 'write')"
              variant="primary"
              :icon="HandCoins"
              :to="{ path: '/payments/new', query: { type: 'PAID', party: po.supplierId, ref: id } }"
            >
              سداد
            </AppButton>
          </template>
        </template>
      </PageHeader>

      <div v-if="po?.missingSupplierInvoice" class="mb-4 flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2.5 text-xs text-warning">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" />
        <span>رقم فاتورة المورد وتاريخها غير مدخلين بعد.</span>
      </div>
      <div v-if="po?.duplicateInvoiceWarning" class="mb-4 flex items-start gap-2 rounded-lg border border-danger/40 bg-danger/10 px-3 py-2.5 text-xs text-danger">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" />
        <span>{{ po.duplicateInvoiceWarning }}</span>
      </div>
      <div v-if="po?.vatNotRecoverable" class="mb-4 flex items-start gap-2 rounded-lg border border-border bg-surface px-3 py-2.5 text-xs text-text-secondary">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" />
        <span>المورد بدون رقم ضريبي — لم تُحتسب ضريبة المدخلات مستردة، وأُضيفت إلى تكلفة البضاعة.</span>
      </div>

      <div class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
        <div class="space-y-5">
          <AppCard padding="none">
            <div v-if="!po" class="p-4"><SkeletonBlock :lines="5" /></div>
            <table v-else class="w-full text-body">
              <thead class="bg-surface text-xs text-text-secondary">
                <tr class="border-b border-border">
                  <th class="px-4 py-2.5 text-start font-medium">الصنف</th>
                  <th class="px-3 py-2.5 text-start font-medium">الكمية</th>
                  <th class="px-3 py-2.5 text-start font-medium">المرتجع</th>
                  <th class="px-3 py-2.5 text-start font-medium">سعر التكلفة</th>
                  <th class="px-4 py-2.5 text-start font-medium">الإجمالي</th>
                </tr>
              </thead>
              <tbody class="bg-background">
                <tr v-for="l in po.lines" :key="l.productId" class="border-b border-border last:border-0">
                  <td class="px-4 py-2.5">
                    <RouterLink :to="`/products/${l.productId}`" class="hover:text-primary">{{ po.products[l.productId]?.name }}</RouterLink>
                    <span class="num block text-tiny text-text-secondary">{{ po.products[l.productId]?.sku }}</span>
                  </td>
                  <td class="px-3 py-2.5"><span class="num">{{ formatNumber(l.qty) }}</span></td>
                  <td class="px-3 py-2.5">
                    <span v-if="po.returnedQty[l.productId]" class="num text-danger">{{ formatNumber(po.returnedQty[l.productId]) }}</span>
                    <span v-else class="text-text-secondary">—</span>
                  </td>
                  <td class="px-3 py-2.5"><MoneyText :value="l.costPrice" plain /></td>
                  <td class="px-4 py-2.5"><MoneyText :value="l.qty * l.costPrice" /></td>
                </tr>
              </tbody>
            </table>
          </AppCard>

          <AppCard v-if="po?.returns.length" title="المرتجعات للمورد" padding="none">
            <ul class="divide-y divide-border text-body">
              <li v-for="r in po.returns" :key="r.id" class="flex items-center justify-between px-4 py-2.5">
                <div>
                  <span class="num font-medium">{{ r.number }}</span>
                  <span class="ms-2 text-text-secondary">{{ r.reason }}</span>
                  <span class="num block text-xs text-text-secondary">{{ formatDateTime(r.date) }}</span>
                </div>
                <MoneyText :value="r.grandTotal" class="text-danger" />
              </li>
            </ul>
          </AppCard>

          <AppCard v-if="po?.payments.length" title="الدفعات" padding="none">
            <ul class="divide-y divide-border text-body">
              <li v-for="p in po.payments" :key="p.id" class="flex items-center justify-between px-4 py-2.5">
                <div>
                  <span class="num font-medium">{{ p.number }}</span>
                  <span class="ms-2 text-text-secondary">{{ PAYMENT_METHOD_LABEL[p.method] }}</span>
                  <span class="num block text-xs text-text-secondary">{{ formatDateTime(p.date) }}</span>
                </div>
                <MoneyText :value="p.amount" />
              </li>
            </ul>
          </AppCard>
        </div>

        <div class="space-y-4">
          <AppCard title="الملخص" padding="sm">
            <SkeletonBlock v-if="!po" :lines="5" />
            <dl v-else class="space-y-1.5 text-body">
              <div class="flex justify-between"><dt class="text-text-secondary">المجموع</dt><dd><MoneyText :value="po.subTotal" /></dd></div>
              <div class="flex justify-between"><dt class="text-text-secondary">الضريبة <span class="num">({{ formatNumber(po.taxRate) }}%)</span></dt><dd><MoneyText :value="po.taxAmount" /></dd></div>
              <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="po.grandTotal" /></dd></div>
              <div v-if="po.returnedAmount" class="flex justify-between text-danger"><dt>المرتجع</dt><dd>−<MoneyText :value="po.returnedAmount" /></dd></div>
              <div class="flex justify-between"><dt class="text-text-secondary">المدفوع</dt><dd><MoneyText :value="po.paidAmount" /></dd></div>
              <div class="flex justify-between font-medium"><dt>المتبقي للمورد</dt><dd><MoneyText :value="po.outstanding" /></dd></div>
            </dl>
            <p v-if="po?.note" class="mt-3 border-t border-border pt-2 text-xs text-text-secondary">{{ po.note }}</p>
          </AppCard>
          <AppCard v-if="po?.landedCosts?.length" title="تكاليف إضافية" padding="none">
            <ul class="divide-y divide-border text-body">
              <li v-for="lc in po.landedCosts" :key="lc.id" class="flex items-center justify-between px-4 py-2.5">
                <span>{{ lc.label }}</span>
                <MoneyText :value="lc.amount" />
              </li>
            </ul>
          </AppCard>
          <AppCard v-if="po?.journalEntries.length && auth.can('accounting')" title="القيود المحاسبية" padding="none">
            <ul class="divide-y divide-border text-body">
              <li v-for="e in po.journalEntries" :key="e.id">
                <RouterLink :to="`/accounting/journal/${e.id}`" class="flex items-center gap-2 px-4 py-2 hover:bg-surface-hover">
                  <BookOpen class="size-3.5 shrink-0 text-text-secondary" />
                  <span class="num text-primary">{{ e.number }}</span>
                  <span class="truncate text-xs text-text-secondary">{{ e.description }}</span>
                </RouterLink>
              </li>
            </ul>
          </AppCard>
          <p v-if="po?.status === 'DRAFT'" class="text-xs text-text-secondary">المسودة لا تؤثر على المخزون أو الحسابات حتى تأكيدها.</p>
        </div>
      </div>
    </template>
  </div>
</template>
